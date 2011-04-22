#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

//gdb support code
//gdb-support.cpp
//this is for the x86 processor with the doors operating system
#include "serial.h"

EXTERNC void flush_i_cache()
{
}

EXTERNC void exceptionHandler (int exception_number, void *exception_address)
{
}

EXTERNC int getDebugChar()
{
	return kellogs.read_serial();
}

EXTERNC void putDebugChar(int put_me)
{
	kellogs.write_serial(put_me);
}
