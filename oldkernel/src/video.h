#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

#ifndef _VIDEO_H_
#define _VIDEO_H_

#ifdef __cplusplus
class video
{
	public:
		video();
		void display(const char *);
		void PrintNumber(unsigned int);
		void clear_screen();
		void put(unsigned char);
	private:
		unsigned int off;
		unsigned int pos;
		void scroll_screen();
};
#endif

EXTERNC void display(const char * cp);
	//this will be called from out ASM code
EXTERNC void PrintNumber(unsigned int bob);
	//this prints an unsigned int number to the screen in hexadecimal
EXTERNC void put(unsigned char);
//prints a single character to the screen
EXTERNC void clear_screen();
//clears the screen

#endif
