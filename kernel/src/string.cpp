//string.cpp
//this handles all string operations for the kernel
#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

#include <stdarg.h>

//#include <stdint.h>
#include "entrance.h"

#include "video.h"
#include "string.h"



EXTERNC int vsprintf (char * str, const char * format, va_list arg )
{
	return -1;
}

EXTERNC int sprintf( char *buffer, const char *format, ... )
{
	return -1;
}

EXTERNC int printf(const char * format, ...)
{	//page 274 of the C99 document
	va_list arg;
	va_start(arg, format);
	//type var_name = va_arg(arg,type);

	char lr_justify;	//-,(d)
	char force_sign;	//+,(d)
	char prefix;	//o,x,X,(d)
	char left_pad;	//0, ' ', (d)
	unsigned int width;
	unsigned int wdigits_given;
	unsigned int precision;
	unsigned int wprec_given;
	char length;	//h,l,L,(d)
	char specifier;	//c,d,i,e,E,f,g,G,o,s,u,x,X,p,n,%,(d)

	int pfchar;	//gcc says char gets promoted to int when passed through ...
	char *pfcharp;
	signed int *nothing;	//this is used for the 'n' specifier

	int num_printed = 0;	//this will be the return value

	size_t offset = 0;
	while ( format[offset] != 0 )
	{	while ( (format[offset] != '%') && ( format[offset] != 0 ) )
		{	put(format[offset]);
			num_printed++;
			offset++;
		}
		if ( format[offset] == '%')
		{	//process format specifiers
			offset++;
			//reset flags
			lr_justify = 'd';	force_sign = 'd';	prefix = 'd';
			left_pad = 'd';		width = 1;			wdigits_given = 0;
			precision = 1;		wprec_given = 0;	length = 'd';
			specifier = 'd';
			//check for flags first (there can be more than one)
			while (format[offset] == '-' || format[offset] == '+' || 
					format[offset] == ' ' || format[offset] == '#' || 
					format[offset] == '0')
			{	switch (format[offset])
				{	case '-': 	lr_justify = '-';	offset++;	break;	//left justify
					case '+':	force_sign = '+';	offset++;	break;	//signed conversion always has a sign
					case ' ':	force_sign = ' ';	offset++;	break;	//signed conversion has a space or a -
					case '0':	left_pad = '0';		offset++;	break;
						//d,i,o,u,x,X,a,A,e,E,f,F,g,G	pad with zeros between sign and number
						//if - is also present, ignore this flag
						//for d,i,o,u,x,X if precision is specified, ignore this flag
					case '#':	prefix = '#';		offset++;	break;	//alternative form
						//	
					default:	break;
				}
			}	//end check for flags
			//check for width specifiers (if *, read one of the variable arguments as the width
			while ( ((format[offset] >= '0') && (format[offset] <= '9')) ||
					(format[offset] == '*') )
			{	if (format[offset] == '*')
				{	//read off a variable argument for the width (TODO: find the proper type for this)
					width = va_arg(arg, int);
					offset++;
				}
				else	//process each digit
				{	if (wdigits_given == 0)
					{	width = format[offset] - '0';
						wdigits_given = 1;
					}
					else
					{	width *= 10;
						width += (format[offset] - '0');
						wdigits_given++;
					}
					offset++;
				}
				wdigits_given = 1;	//width is specified
			}	//end of check for width
			//check for precision qualifiers
			if (format[offset] == '.')
			{	//a precision specifier is present
				offset++;	//go to the next char
				precision = 0;	//if no number is given, 0 is assumed
				while ( ((format[offset] >= '0') && (format[offset] <= '9')) ||
						(format[offset] == '*') )
				{	if (format[offset] == '*')
					{	//read off a variable argument for the precision (TODO: find the proper type for this)
						precision = va_arg(arg, int);
						offset++;
					}
					else	//process each digit
					{	if (wprec_given == 0)
						{	precision = format[offset] - '0';
							wprec_given = 1;
						}
						else
						{	precision *= 10;
							precision += (format[offset] - '0');
							wprec_given++;
						}
						offset++;
					}
				}
				wprec_given = 1;	//set to 1 after it is done being used, this means that precision is specified
			}	//end of check for precision qualifiers
			//check for length specifiers (if more than one can be specified, then this needs to be enclosed in a while loop
			switch (format[offset])
			{	//hh(i), h, l, ll(m), j, z, t, L, (d)
				case 'h':
 					length = 'h';	//d,i,o,u,x,X short int or unsigned short int n->short int*
					offset++;
					if (format[offset] == 'h')
						length = 'i';	//d,i,o,u,x,X (un)signed char n->signed char*
					break;
				case 'l':
					length = 'l';	//d,i,o,u,x,X long int or unsigned long int 
					offset++;		//n->long int*	c->wint_t	s->wchar_t*
					if (format[offset] == 'l')
						length = 'm';	//d,i,o,u,x,X long long int or unsigned long long int
					break;				//n->long long int*
				case 'j':	length = 'j';	offset++;	break;	//d,i,o,u,x,X intmax_t, uintmax_t or n->intmax_t
				case 'z':	length = 'z';	offset++;	break;	//d,i,o,u,x,X size_t
					//n is a pointer to a signed size_t*
				case 't':	length = 't';	offset++;	break;	//d,i,o,u,x,X ptrdiff_t n->ptrdiff_t*
				case 'L':	length = 'L';	offset++;	break;	//a,A,e,E,f,F,g,G long double
				default:	break;
			}	//end length specifers check
			//now we check for specifiers (c,d,i,e,E,f,g,G,o,s,u,x,X,p,n,%) and then parse flags
			switch (format[offset])
			{
				case '%':	put('%');	num_printed++;	offset++;	break;
				case 'c':	//character
					if (length == 'l')
					{	//wchar_t
						//find the length of the wchar
						//print the wchar
						//print padding (or the other way around for the previous line)
					}
					else
					{	//char (gcc says it gets promoted to int... weird)
						pfchar = va_arg(arg, int);
						if (lr_justify == '-')
						{	//left justify
							put(pfchar);
							num_printed++;
							for (unsigned int count = 1; count < width; count++)
							{
								put(' ');
								num_printed++;
							}
						}
						else
						{	//right justify
							for (unsigned int count = 1; count < width; count++)
							{
								put(' ');
								num_printed++;
							}
							put(pfchar);
							num_printed++;
						}
					}
					offset++;
					break;
				case 's':	//strings
					if (length == 'l')
					{	//wchar_t*
						//find the length of the wchar
						//print the wchar
						//print padding (or the other way around for the previous line)
					}
					else
					{	//char* "abcdef"(order from smallest to largest)
						pfcharp = va_arg(arg, char*);
						if ( wprec_given && (precision < strlen(pfcharp)) )
						{	//precision matters (string will be cut off)
							if (wdigits_given && (width > strlen(pfcharp)) )
							{	//width matters (cut off string will be padded, figure out which side)
							}
							else
							{	//width doesn't matter (cut off string will be printed, no padding)
								for (unsigned int temp = 0; temp < precision; temp++)
								{	put(pfcharp[temp]);
									num_printed++;
								}
							}
						}
						else
						{	//precision doesn't matter
							if (wdigits_given && (width > strlen(pfcharp)) )
							{	//width matters (padding is required, figure out what side)
								
							}
							else
							{	//width doesn't matter
								for (unsigned int temp = 0; pfcharp[temp]; temp++)
								{	put(pfcharp[temp]);
									num_printed++;
								}
							}
						}
					}
					offset++;
					break;
				default:
					offset++;
					break;
			}
		}
	}
	va_end(arg);
	return offset;
}

EXTERNC int strlen(char *string)
{
	int counter;
	for (counter = 0; string[counter] != '\0'; counter++) {};
	return counter;
}

EXTERNC int strlenw(unsigned short *string)
{	//works on two-byte characters
	int counter;
	for (counter = 0; string[counter] != 0xFFFF; counter++) {};
	return counter;
}

EXTERNC char *strcpy(char *destination, const char *source )
{
	int counter = 0;
	do
	{
		destination[counter] = source[counter];
		counter++;
	} while (source[counter - 1] != 0);
	return destination;
}

/*EXTERNC short *strcpyw(short *destination, const short *source)
{	//works on two-byte characters
	int counter = 0;
	do
	{
		destination[counter] = source[counter];
		counter++;
	} while (source[counter - 1] != 0);
	return destination;
}*/

EXTERNC unsigned int stringCompare(const char *a, const char *b)
{	//TODO: lookup the specifications for strcmp and implement it
	for(unsigned int bob = 0; ( (a[bob] != '\0') || (b[bob] != '\0') );bob++)
	{
		if (a[bob] != b[bob])
			return -1;
	}
	return 0;
}

EXTERNC unsigned short *precatenatew(unsigned short *original, unsigned short *insert)
{
	unsigned short *ret_val;
	unsigned int a, b;
	ret_val = new unsigned short[strlenw(insert) + strlenw(original) + 1];
		//declare enough size to place both string and a terminator into it
	for (a = 0; insert[a] != 0xFFFF; a++)
	{
		ret_val[a] = insert[a];
	}
	for (b = 0; original[b] != 0xFFFF; b++, a++)
	{
		ret_val[a] = original[a];
	}
	ret_val[a] = 0xFFFF;
	return ret_val;	
}
