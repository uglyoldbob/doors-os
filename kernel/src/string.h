//string.h
//this handles string operation definitions for the kernel and related stuff
#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 
#include <stddef.h>

EXTERNC int strlen(char *);
	//returns the number of characters in the string minus the null terminator
	//TODO: look up the actual argument types, until then the ones i have selected should be sufficient

EXTERNC void *memset(void *s, int c, size_t n);

EXTERNC char *strcpy(char *destination, const char *source );
	//TODO: lookup return value

EXTERNC unsigned int stringCompare(const char *a, const char *b);
	//returns 0 if the strings are equal, returns -1 if they are not the same
	//strings must be null terminated



EXTERNC int strlenw(unsigned short *);
	//returns the number of characters in the string minus the null terminator
	//TODO: look up the actual argument types, until then the ones i have selected should be sufficient

//EXTERNC unsigned short *strcpyw(unsigned short *destination, const unsigned short *source );
	//TODO: lookup return value

EXTERNC unsigned short *precatenatew(unsigned short *original, unsigned short *insert);
