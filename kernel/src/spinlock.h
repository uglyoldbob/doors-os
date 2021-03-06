#ifdef __cplusplus
#define EXTERNC extern "C"
#else
#define EXTERNC
#endif 

//most interruptable
#define SL_BLANK 	0	//don't use
#define SL_DISPLAY 	1
#define SL_MEM_MNG 	2
#define SL_IRQ1 	3
#define SL_MESSAGE 	4	//message pumping system
//least interruptable

//lower levels can call functions involved in higher levels
//must recompile everything if these defines are changed
#define NUMBER_TYPES 5

struct SL_STATES
{
	unsigned long exp_enabled;		//this spinlock was explicitly entered
	unsigned long imp_enabled;	//this spinlock was not entered, but a higher level was entered first
	unsigned long delays;				//number of times this spinlock spun
};

EXTERNC void enter_spinlock(unsigned int which);
//enters a spinlock level, if rules allow
EXTERNC void leave_spinlock(unsigned int which);
//leaves a spinlock level
EXTERNC void setup_spinlock_data();
///sets up data structure for spinlocks to work
EXTERNC void initialize_spinlock();
//must be called before interrupts and IRQ's are enabled
