#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

EXTERNC void setupPIC();//sets up the PIC and then enables interrupts
EXTERNC void setupTimer(unsigned int frequency);	//changes the frequency of IRQ0, the timer
EXTERNC void clearIRQ(unsigned int which);	//disables irq's
EXTERNC void enableIRQ(unsigned int which);	//enables irq's
