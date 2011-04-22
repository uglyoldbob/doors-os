extern unsigned int inportb(unsigned int port);	
extern unsigned int outportb(unsigned int value, unsigned int port);
extern unsigned int inportw(unsigned int port);	
extern unsigned int outportw(unsigned int value, unsigned int port);
extern unsigned int getResponse();	//waits for and retrieves a byte response from the keyboard
extern volatile unsigned int HD_INTS;

extern unsigned int timer;
extern unsigned long getCR3();

extern void WaitKey();
	//waits for the pause/break key to be pressed
extern void EnablePaging(unsigned int address);
	//assembly code to signal the processor to enable paging
extern void Delay(unsigned int);
	//delays for mmmm milliseconds

extern unsigned int test_and_set (unsigned long new_value, unsigned long *lock_pointer);
	//use this to set an (unsigned int) that is shared
	//will not return unless the two final values are different
