#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

EXTERNC volatile unsigned int inportb(unsigned int port);	
EXTERNC unsigned int outportb(unsigned int value, unsigned int port);
EXTERNC volatile unsigned int inportw(unsigned int port);	
EXTERNC unsigned int outportw(unsigned int value, unsigned int port);
EXTERNC unsigned int getResponse();	//waits for and retrieves a byte response from the keyboard
EXTERNC unsigned int getEIP();
EXTERNC volatile unsigned int HD_INTS;
EXTERNC unsigned int setup_multi_gdt();

EXTERNC unsigned int timer;
EXTERNC unsigned long getCR3();
EXTERNC unsigned long invlpg_asm(unsigned long address);
	//invalidates the TLB buffer for the specified address

EXTERNC void WaitKey();
	//waits for the pause/break key to be pressed
EXTERNC void EnablePaging(unsigned int address);
	//assembly code to signal the processor to enable paging
EXTERNC void Delay(unsigned int);
	//delays for mmmm milliseconds

EXTERNC unsigned int test_and_set (unsigned long new_value, unsigned long *lock_pointer);
	//use this to set an (unsigned int) that is shared
	//will not return unless the two final values are different
