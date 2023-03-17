#ifndef __DOORS_STDIO_H_
#define __DOORS_STDIO_H_

#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

#include <stdarg.h>

EXTERNC int vsprintf (char * str, const char * format, va_list arg );

EXTERNC int sprintf( char *buffer, const char *format, ... );

EXTERNC int printf(const char * format, ...);

EXTERNC int putchar(int c);

typedef void * FILE;

EXTERNC int fgetc(FILE *stream);

FILE * stdin;

#endif
