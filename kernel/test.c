//this file is not officially part of the source for doors.
//this is a test source for testing various functions (like printf)
#include <stdio.h>

int main()
{
	printf ("Characters: %c %c \n", 'a', 65);
	printf ("Decimals: %d %ld\n", 1977, 650000);
	printf ("Preceding with blanks: %10d \n", 1977);
	printf ("Preceding with zeros: %010d \n", 1977);
	printf ("Some different radixes: %d %x %o %#x %#o \n", 100, 100, 100, 100, 100);
	printf ("floats: %4.2f %+.0e %E \n", 3.1416, 3.1416, 3.1416);
	printf ("Width trick: %*d \n", 5, 10);
	printf ("%s \n", "A string");

	printf ("Test printing of characters\n");	
	printf ("TEST:%%%+5c%%\n", 'a');
	printf ("TEST:%%%-5c%%\n", 'a');
	printf ("TEST:%%% 5c%%\n", 'a');
	printf ("TEST:%%%05c%%\n", 'a');
	printf ("TEST:%%%#5c%%\n", 'a');
	printf ("TEST:%%%05c%%\n", 'a');	
	
	printf ("Test printing of strings\n");
	printf ("TEST:%%%-5.1s%%\n", "asdfghjk");
	printf ("TEST:%%%s%%\n", "asdfghjk");
	printf ("TEST:%%%s%%\n", "asdfghjk");
	printf ("TEST:%%%s%%\n", "asdfghjk");
	printf ("TEST:%%%s%%\n", "asdf");
	printf ("TEST:%%%-5s%%\n", "asdf");
	return 0;
}
