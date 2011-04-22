//Support.c
void _main()
{
//	//Walk and call the constructors in the ctor_list
//	//the ctor list is defined in the linker script
//	extern void (*__CTOR_LIST__)() ;
//	//hold current constructor in list
//	void (**constructor)() = &__CTOR_LIST__ ;
//	//the first int is the number of constructors
//	int total = *(int *)constructor ;
//	//increment to first constructor
//	constructor++ ;
//	while(total)
//	{
//		(*constructor)() ;
//		total-- ;
//		constructor++ ;
//	}
}

void _atexit()
{
//	///Walk and call the deconstructors in the dtor_list
//	//the dtor list is defined in the linker script
//	extern void (*__DTOR_LIST__)() ;
//	//hold current deconstructor in list
//	void (**deconstructor)() = &__DTOR_LIST__ ;
//	//the first int is the number of deconstructors
//	int total = *(int *)deconstructor ;
//	//increment to first deconstructor
//	deconstructor++ ;
//	while(total)
//	{
//		(*deconstructor)() ;
//		total-- ;
//		deconstructor++ ;
//	}
}
/*
void __cxa_pure_virtual()
{	//provides support for pure vivrtual functions in C++
	// print error message
}

int __cxa_atexit(void (*f)(void *), void *p, void *d);
void __cxa_finalize(void *d);

void *__dso_handle; /*only the address of this symbol is taken by gcc*/
/*
struct object
{
        void (*f)(void*);
        void *p;
        void *d;
} object[32] = {0};
unsigned int iObject = 0;

int __cxa_atexit(void (*f)(void *), void *p, void *d)
{
        if (iObject >= 32) return -1;
        object[iObject].f = f;
        object[iObject].p = p;
        object[iObject].d = d;
        ++iObject;
        return 0;
}

/* This currently destroys all objects */
/*void __cxa_finalize(void *d)
{
        unsigned int i = iObject;
        for (; i > 0; --i)
        {
                --iObject;
                object[iObject].f(object[iObject].p);
        }
}
*/
