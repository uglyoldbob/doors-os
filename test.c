#include <stdio.h>

int main()
{
	FILE *test;
	test = fopen("stupid.txt", "w");
	putc('x', test);
	putc('y', test);
	putc('z', test);
	fclose(test);
    printf("Hello world\n");
    return 0;
}
