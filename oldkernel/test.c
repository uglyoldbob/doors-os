//this file is not officially part of the source for doors.
//this is a test source for testing various functions (like printf)
#include <stdio.h>
#include <stdint.h>

int main()
{
	printf(", %i\n", printf ("Characters: %c %c", 'a', 65));
	printf(", %i\n", printf ("Decimals: %d %ld", 1977, 650000));
	printf(", %i\n", printf ("Preceding with blanks: %10d", 1977));
	printf(", %i\n", printf ("Preceding with zeros: %010d", 1977));
	printf(", %i\n", printf ("Some different radixes: %d %x %o %#x %#o", 100, 100, 100, 100, 100));
	printf(", %i\n", printf ("floats: %4.2f %+.0e %E", 3.1416, 3.1416, 3.1416));
	printf(", %i\n", printf ("Width trick: %*d", 5, 10));
	printf(", %i\n", printf ("%s", "A string"));

	printf ("0         1         2         3         4         5         6         7\n");
	printf ("01234567890123456789012345678901234567890123456789012345678901234567890\n");	
	printf ("Test functionality of c\n");
	printf(", %i\n", printf ("TEST:%%%c%%", 'a'));
	printf(", %i\n", printf ("TEST:%%%-5c%%", 'a'));
	printf(", %i\n", printf ("TEST:%%%+5c%%", 'a'));
	printf(", %i\n", printf ("TEST:%%%#5c%%", 'a'));
	printf(", %i\n", printf ("TEST:%%% 5c%%", 'a'));
	printf(", %i\n", printf ("TEST:%%%05c%%", 'a'));
	printf(", %i\n", printf ("TEST:%%%5.2c%%", 'a'));
	
	printf ("Test printing of strings\n");
	printf(", %i\n", printf ("TEST:%%%-5.1s%%", "asdfghjk"));
	printf(", %i\n", printf ("TEST:%%%s%%", "asdfghjk"));
	printf(", %i\n", printf ("TEST:%%%s%%", "asdf"));
	printf(", %i\n", printf ("TEST:%%%-5s%%", "asdf"));

	printf(", %i\n", printf ("Test printing of decimals (d) and integers (i)\n"));
	printf(", %i\n", printf ("%%%- d%%", 12345));
	printf(", %i\n", printf ("%%%12d%%", 12345));

	printf("Sizeof (intmax_t): %i\n", sizeof(intmax_t));
	return 0;
}
