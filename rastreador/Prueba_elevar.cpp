#include <iostream>
#include <cmath>

using namespace std;

int main() {
    double base, exponent, result;
    cout << "Ingrese la base: ";
    cin >> base;
    cout << "Ingrese el exponente: ";
    cin >> exponent;
    result = pow(base, exponent);

    cout << base << " a la " << exponent << " es: " << result << endl;

    return 0;
}