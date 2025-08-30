use std::{
    collections::HashMap, //The hashmap will be used to storage the json
    io::{self, Write, Read},
    os::unix::process::CommandExt, //UNIX extensions to process programs
    process::Command, //Execute system commands
};
use nix::{ //INTERFACES FOR UNIX
           sys::{ptrace, wait::waitpid}, //ptrace is key for tracing the processes
           unistd::Pid,
};
use owo_colors::OwoColorize; //Add color to the terminal so it looks better
use clap::Parser; //Analyse line commands arguments

#[derive(Parser)] //Makes it so it analyses the arguments automatically
//  disable_version_flag to prevent conflict with auto-generated -V flag
#[command(version, about, long_about = None, disable_version_flag = true)]
struct Arguments_struct {
    /// Main Program to trace, It'll be represented as a String
    main_program_run: String,

    /// Arguments necessary so the program runs
    arguments_of_program: Vec<String>,

    /// system call show, it shows the system call details in a run
    #[arg(short = 'v', long)]
    system_call_show: bool,

    /// System_call_key its a flag for the more detailed mode that requires the press of a key to continue
    #[arg(short = 'V', long = "system_call_key")]
    system_call_key: bool,
}
//function to wait for the key
fn wait_for_key() {
    let mut stdout = io::stdout();
    let _ = stdout.write_all(b"Press ANY key for next step");
    let _ = stdout.flush(); //makes message show inmediatly

    let mut stdin = io::stdin();
    let mut buffer = [0; 1];
    let _ = stdin.read_exact(&mut buffer);
}
//main function :)
//Box so it can show an error
fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse  arguments
    let arguments = Arguments_struct::parse();

    // Check for the two options
    if !arguments.system_call_show && !arguments.system_call_key {
        eprintln!("No option inputted neither -v or -V");
        std::process::exit(1);
    }

    eprintln!("Tracing program: {} with args: {:?}", arguments.main_program_run, arguments.arguments_of_program);

    // Load the JSON system calls so it knows what is it doing on a Hashmap
    let json: serde_json::Value = serde_json::from_str(include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/syscall.json")))?;
    let syscall_table_from_json: HashMap<u64, String> = json["aaData"]
        .as_array()
        .unwrap()
        .iter()
        .map(|item| {
            (
                item[0].as_u64().unwrap(),
                item[1].as_str().unwrap().to_owned(),
            )
        })
        .collect();

    // The child command will be the one to Trace so it need to be prepared
    let mut command = Command::new(&arguments.main_program_run);
    command.args(&arguments.arguments_of_program);

    // Make child process actuallty be traceable
    unsafe { //not really that safe
        command.pre_exec(|| { //runs everything before the child process
            nix::sys::ptrace::traceme().map_err(|e| e.into())  //with ptrace the father process is capable of tracing the child process
        });
    }

    // Start child process
    let the_child = command.spawn()?;
    let the_child_pid = Pid::from_raw(the_child.id() as i32); //makes it a Pid for traces

    // Wait for initial setup
    let wait_child = waitpid(the_child_pid, None)?; //waits for the child to change state
    eprintln!("Initial wait: {:?}", wait_child.yellow());

    // Start tracing system calls
    let mut is_systemcall_exit = false;
    let unknown_str = "unknown".to_string();

    // HashMap to count system calls
    let mut syscall_counts: HashMap<String, u32> = HashMap::new();

    // Main tracing loop - continues until child process exits
    loop {
        // Execute until next syscall
        ptrace::syscall(the_child_pid, None)?; // Tell child to continue until next system call

        // Wait for the child to stop the systemcall
        let status = waitpid(the_child_pid, None)?; // Wait for child to stop

        // check if the child process exited
        if let nix::sys::wait::WaitStatus::Exited(..) = status {
            break; // Exit loop if child process has terminated
        }

        // Print the system call information on exit
        if is_systemcall_exit {
            let registers = ptrace::getregs(the_child_pid)?; // Get CPU registers of child process

            // Get system call name and links it to the json hashmap
            let syscall_name = syscall_table_from_json
                .get(&registers.orig_rax) // orig_rax contains the system call number
                .unwrap_or(&unknown_str)
                .clone();

            // Count the system call
            *syscall_counts.entry(syscall_name.clone()).or_insert(0) += 1;

            // Print system call details, these registers contain the first  arguments of the syscall
            eprintln!(
                "{}({:x}, {:x}, {:x}, ...) = {:x}",
                syscall_name.blue(),
                registers.rdi.green(),  // First argument
                registers.rsi.green(),  // Second argument
                registers.rdx.green(),  // Third argument
                registers.rax.red(), //return value of the syscall
            );

            // Pause if -V option is enabled
            if arguments.system_call_key {
                wait_for_key(); // Wait for user input before continuing
            }
        }

        is_systemcall_exit = !is_systemcall_exit; // Toggle between system call entry and exit
    }

    // Display cumulative system call table
    // Use stdout instead of stderr for the table
    let mut stdout = io::stdout();
    writeln!(stdout, "\n{}", "=== TABLA  DE SYSTEM CALLS ===".bold().green())?;
    writeln!(stdout, "{:<20} {:<10}", "System Call", "Cantidad")?;
    writeln!(stdout, "{}", "=".repeat(32))?;

    let mut syscall_vec: Vec<(&String, &u32)> = syscall_counts.iter().collect();
    syscall_vec.sort_by(|a, b| b.1.cmp(a.1)); // Sort by count (descending)

    for (syscall, count) in syscall_vec {
        writeln!(stdout, "{:<20} {:<10}", syscall, count)?;
    }

    let total_syscalls: u32 = syscall_counts.values().sum();
    writeln!(stdout, "{}", "=".repeat(32))?;
    writeln!(stdout, "{:<20} {:<10}", "Total", total_syscalls)?;
    // Ensure the table is flushed to output
    stdout.flush()?;

    Ok(()) // Return success
}