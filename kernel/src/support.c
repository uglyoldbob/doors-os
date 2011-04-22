#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif

//Support.c
#include "video.h"

EXTERNC void _main()
{
	//Walk and call the constructors in the ctor_list
	//the ctor list is defined in the linker script
	extern void (*__CTOR_LIST__)() ;
	//hold current constructor in list
	void (**constructor)() = &__CTOR_LIST__ ;
	//the first int is the number of constructors
	int total = *(int *)constructor ;
	//increment to first constructor
	constructor++ ;
	while(total)
	{
		(*constructor)() ;
		total-- ;
		constructor++ ;
	}
}

EXTERNC void __cxa_pure_virtual()
{
	display("Pure virtual function called\n");
	while (1);
}

/*
EXTERNC
{
	int __cxa_atexit(void (*f)(void *), void *p, void *d);
	void __cxa_finalize(void *d);
};*/

void *__dso_handle; /*only the address of this symbol is taken by gcc*/

struct object
{
	void (*f)(void*);
	void *p;
	void *d;
} object[32] = {0};
unsigned int iObject = 0;

EXTERNC int __cxa_atexit(void (*f)(void *), void *p, void *d)
{
	if (iObject >= 32)
		return -1;
	object[iObject].f = f;
	object[iObject].p = p;
	object[iObject].d = d;
	++iObject;
	return 0;
}

/* This currently destroys all objects */
EXTERNC void __cxa_finalize(void *d)
{
	unsigned int i = iObject;
	for (; i > 0; --i)
	{
		--iObject;
		object[iObject].f(object[iObject].p);
	}
}
