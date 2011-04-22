unsigned int pos;
unsigned int off;

void display(char * chr);
	//this will be called from out ASM code
void PrintNumber(unsigned int bob);
	//this prints an unsigned int number to the screen in hexadecimal
void put(unsigned char);
//prints a single character to the screen
void clear_screen();
//clears the screen
void scroll_screen();
//scrolls the screen up one line
