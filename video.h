unsigned long pos;
unsigned long off;

void display(char * chr);
	//this will be called from out ASM code
void PrintNumber(unsigned long bob);
	//this prints an unsigned long number to the screen in hexadecimal
void put(unsigned char);
//prints a single character to the screen
void clear_screen();
//clears the screen
void scroll_screen();
//scrolls the screen up one line
