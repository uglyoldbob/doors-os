#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif

#include <stdio.h>
#include <stddef.h>
#include <stdint.h>
#include <string.h>
#include "video.h"

EXTERNC int vsprintf (char * str, const char * format, va_list arg )
{
	return -1;
}

EXTERNC int sprintf( char *buffer, const char *format, ... )
{
	int ret;
	va_list arg;
	va_start(arg, format);
	ret = vsprintf(buffer, format, arg);
	va_end(arg);
	return ret;
}

//insert wdth padding (pad symbol, number to insert)
	//no return value needed
void printf_wdthpad(char sym, ptrdiff_t number, size_t *num_written)
{
	for (ptrdiff_t count = 0; count < number; count++)
	{
		put(sym);
		*num_written += 1;
	}
}

void printf_signi(char force_sign, intmax_t pf_arg, size_t *num_written)
{	//according to the standard, intmax_t is a signed integer type capable of
		//representing any value of any signed integer type
	if (pf_arg >= 0)
	{	if	(force_sign != 'd')
		{
			put(force_sign);
			*num_written += 1;
		}
	}
	else
	{
		put('-');
		*num_written += 1;
	}
}

ptrdiff_t printf_cmp(int lesser, ptrdiff_t a, ptrdiff_t b)
{	//returns the lesser/greater of two numbers
	if (lesser)
	{
		if (a < b)
			return a;
		return b;
	}
	else
	{
		if (a > b)
			return a;
		return b;
	}
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
	ptrdiff_t wdth;
	unsigned int wdigits_given;
	ptrdiff_t prcsn ;
	unsigned int wprec_given;
	char length;	//h,l,L,(d)
	char specifier;	//c,d,i,e,E,f,g,G,o,s,u,x,X,p,n,%,(d)

	unsigned char pf_char;
	char *pf_charp;
	signed int pf_di;	//used for the d and i specifiers

	size_t num_prntd = 0;	//this will be the return value
	int error = 0;	//set to 1 if an error occurs and -1 will be the return value

	size_t offset = 0;
	while ( format[offset] != 0 )
	{	while ( (format[offset] != '%') && (format[offset]) )
		{	put(format[offset]);
			num_prntd++;
			offset++;
		}
		if ( format[offset] == '%')
		{	//process format specifiers
			offset++;
			//reset flags
			lr_justify = 'd';	force_sign = 'd';	prefix = 'd';	left_pad = ' ';
			wdth = 1;			wdigits_given = 0;	prcsn  = 1;		wprec_given = 0;
			length = 'd';		specifier = 'd';
			//check for flags first (there can be more than one)
			while (format[offset] == '-' || format[offset] == '+' || format[offset] == ' ' ||
					format[offset] == '#' || format[offset] == '0')
			{	switch (format[offset])
				{	case '-': 	lr_justify = '-';	offset++;	break;	//left justify
					case '+':	force_sign = '+';	offset++;	break;	//signed conversion always has a sign
					case ' ':	//if ' ' and '+' are both present, ignore ' '
						if (force_sign != '+')
							force_sign = ' ';
						offset++;
						break;	//signed conversion has a space or a -
					case '0':	left_pad = '0';		offset++;	break;
						//d,i,o,u,x,X,a,A,e,E,f,F,g,G	pad with zeros between sign and number
						//if - is also present, ignore this flag
						//for d,i,o,u,x,X if prcsn  is specified, ignore this flag
					case '#':	prefix = '#';		offset++;	break;	//alternative form
					default:	break;
				}
			}	//end check for flags
			//check for wdth specifiers (if *, read one of the variable arguments as the width
			while ( ((format[offset] >= '0') && (format[offset] <= '9')) ||
					(format[offset] == '*') )
			{	if (format[offset] == '*')
				{	//read off a variable argument for the width
					wdth = va_arg(arg, int);
					offset++;
				}
				else	//process each digit
				{	if (wdigits_given == 0)
					{	wdth = format[offset] - '0';
						wdigits_given = 1;
					}
					else
					{	wdth *= 10;
						wdth += (format[offset] - '0');
						wdigits_given++;
					}
					offset++;
				}
				wdigits_given = 1;	//wdth is specified
			}	//end of check for wdth
			//check for prcsn  qualifiers
			if (format[offset] == '.')
			{	//a prcsn  specifier is present
				offset++;	//go to the next char
				prcsn  = 0;	//if no number is given, 0 is assumed
				while ( ((format[offset] >= '0') && (format[offset] <= '9')) ||
						(format[offset] == '*') )
				{	if (format[offset] == '*')
					{	//read off a variable argument for the prcsn  (TODO: find the proper type for this)
						prcsn  = va_arg(arg, int);
						offset++;
					}
					else	//process each digit
					{	if (wprec_given == 0)
						{	prcsn  = format[offset] - '0';
							wprec_given = 1;
						}
						else
						{	prcsn  *= 10;
							prcsn  += (format[offset] - '0');
							wprec_given++;
						}
						offset++;
					}
				}
				wprec_given = 1;	//set to 1 after it is done being used, this means that prcsn  is specified
			}	//end of check for prcsn  qualifiers
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
				case '%':	put('%');	num_prntd++;	offset++;	break;
				case 'c':	//character
					if (length == 'l')
					{	//wint_t
						error = 1;
					}
					else
					{	//char
						pf_char = (unsigned char)va_arg(arg, int);
						if (lr_justify == '-')
						{	//left justify
							put(pf_char);
							num_prntd++;
							printf_wdthpad(' ', wdth - 1, &num_prntd);
						}
						else
						{//right justify
							printf_wdthpad(' ', wdth - 1, &num_prntd);
							put(pf_char);
							num_prntd++;
						}
					}
					offset++;
					break;
				case 's':	//strings
					if (length == 'l')
					{	//wchar_t*
						//find the length of the wchar, print the wchar, print padding (or the other way around)
						error = 1;
					}
					else
					{	//char*
						pf_charp = va_arg(arg, char*);
						if ( wprec_given && (prcsn  < strlen(pf_charp)) )
						{	//prcsn  matters (string will be cut off)
							if (wdigits_given && (wdth > prcsn ) )
							{	//wdth matters (cut off string will be padded, figure out which side)
								if (lr_justify == '-')
								{	//left justify, padding comes after the string
									ptrdiff_t count = 0;
									for (;count < prcsn ; count++)
									{	put(pf_charp[count]);
										num_prntd++;
									}
									printf_wdthpad(' ', (wdth - prcsn ), &num_prntd);
								}
								else
								{	//right justify, padding comes before the string
									ptrdiff_t count = 0;
									printf_wdthpad(' ', (wdth - prcsn ), &num_prntd);
									for (count = 0;count < prcsn ; count++)
									{	put(pf_charp[count]);
										num_prntd++;
									}
								}
							}
							else
							{	//wdth doesn't matter (cut off string will be printed, no padding)
								for (size_t temp = 0; temp < prcsn ; temp++)
								{	put(pf_charp[temp]);
									num_prntd++;
								}
							}
						}
						else
						{	//prcsn  doesn't matter
							if (wdigits_given && (wdth > strlen(pf_charp)) )
							{	//wdth matters (padding is required, figure out what side)
								if (lr_justify == '-')
								{	//left justify, padding comes after the string
									ptrdiff_t count = 0;
									for (count = 0; pf_charp[count]; count++)
									{	put(pf_charp[count]);
										num_prntd++;
									}
									printf_wdthpad(' ', (wdth - strlen(pf_charp)), &num_prntd);
								}
								else
								{	//right justify
									ptrdiff_t count = 0;
									printf_wdthpad(' ', (wdth - strlen(pf_charp)), &num_prntd);
									for (count = 0; pf_charp[count]; count++)
									{	put(pf_charp[count]);
										num_prntd++;
									}
								}
							}
							else
							{	//wdth doesn't matter
								for (ptrdiff_t temp = 0; pf_charp[temp]; temp++)
								{	put(pf_charp[temp]);
									num_prntd++;
								}
							}
						}
					}
					offset++;
					break;
				case 'd':	case 'i':	//i am pretty sure these are treated the same
				{	//wdth handles the "entire" minimum number of chars to print
					//prcsn  handles the minimum number of "digits" to print
					ptrdiff_t sgn_lngth = 0;	//stores the length of the sign (0 or 1)
					ptrdiff_t num_length = 0;	//stores the length of the printed number before it is printed
					intmax_t base = 10;			//stores the base of the number to be printed
					intmax_t pf_arg;
					switch(length)
					{	//arrrrgh. by using intmax_t, gcc complains about not knowing how to multiply, divide, modulus it
						//TODO: figure out how to get this working, because I think it is a great solution
						case 'i':
							pf_arg = (signed char)va_arg(arg, int);	break;
						case 'h':
							pf_arg = (short int)va_arg(arg, int);	break;
						case 'l':
							pf_arg = va_arg(arg, long int);			break;
						case 'm':
							pf_arg = va_arg(arg, long long int);	break;
						case 'j':
							pf_arg = va_arg(arg, intmax_t);			break;
						case 'z':
							pf_arg = va_arg(arg, size_t);			break;//what is the signed equivalent for size_t?
						case 't':
							pf_arg = va_arg(arg, ptrdiff_t);		break;
						default:
							pf_arg = va_arg(arg, int);				break;
					}
					intmax_t pf_arg2 = pf_arg;
					if (wprec_given == 1)
						left_pad = ' ';
					if ( (pf_arg >= 0) && (force_sign == 'd') )	//find the number of bytes required for the sign
					{	sgn_lngth = 0; }
					else
					{	sgn_lngth = 1; }
					while (pf_arg2)
					{	//find the length of the number
						pf_arg2 = (pf_arg2 / base);
						num_length++;
					}
					//(wdth_pad)(sign)(prec_pad-0)(number)(wdth_pad)
					if (lr_justify == '-')
					{	//left justify (sign)(prec_pad-0)(number)(wdth_pad)
						left_pad = ' ';
						printf_signi(force_sign, pf_arg, &num_prntd);
						printf_wdthpad('0', (prcsn  - num_length), &num_prntd);	//precision
						for(ptrdiff_t position = num_length; position > 0; position--)
						{	//print the number (shouldn't need a signed type because position nevers goes below 0)
							pf_arg2 = pf_arg;
							for (size_t mul = 1; mul < position; mul++)
								pf_arg2 = (intmax_t)(pf_arg2/base);
							pf_arg2 = pf_arg2 % base;
							put(pf_arg2 + '0');
							num_prntd++;
						}
						printf_wdthpad(left_pad, (wdth - (sgn_lngth + printf_cmp(0, prcsn , num_length))), &num_prntd);
					}
					else
					{	//right justify (wdth_pad)(sign)(prec_pad-0)(number)
						printf_wdthpad(left_pad, (wdth - (sgn_lngth + printf_cmp(0, prcsn , num_length))), &num_prntd);
						printf_signi(force_sign, pf_arg, &num_prntd);
						printf_wdthpad('0', (prcsn  - num_length), &num_prntd);	//precision
						for(ptrdiff_t position = num_length; position > 0; position--)
						{	//print the number (shouldn't need a signed type because position nevers goes below 0)
							pf_arg2 = pf_arg;
							for (size_t mul = 1; mul < position; mul++)
								pf_arg2 = (intmax_t)(pf_arg2/base);
							pf_arg2 = pf_arg2 % base;
							put(pf_arg2 + '0');
							num_prntd++;
						}
					}
					offset++;
					break;
				}
				default:	//all unhandled specifiers
					error = 1;
					offset++;
					break;
			}
		}
	}
	va_end(arg);
	if (error)
		return -1;
	return num_prntd;
}

int fgetc(FILE *stream)
{ return -1;	}

