fn wait_for_key() {
    let mut stdout = io::stdout();
    let _ = stdout.write_all(b"Press ANY key for next step");
    let _ = stdout.flush(); 

    let mut stdin = io::stdin();
    let mut buffer = [0; 1];
    let _ = stdin.read_exact(&mut buffer);
}