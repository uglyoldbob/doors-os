void setupPIC();//sets up the PIC and then enables interrupts
void setupTimer(unsigned int frequency);	//changes the frequency of IRQ0, the timer
void clearIRQ(unsigned int which);	//disables irq's
void enableIRQ(unsigned int which);	//enables irq's
