//most interruptable
#define SL_BLANK 0	//don't use
#define SL_DIS2	1
#define SL_DISPLAY 2
#define SL_MEM_MNG 3
#define SL_IRQ1 4
#define SL_MESSAGE 5	//message pumping system
//least interruptable

//lower levels can call functions involved in higher levels
	//memory management can use display functions
//must recompile everything if these defines are changed
#define NUMBER_TYPES 6

struct SL_STATES
{
	unsigned long exp_enabled;		//this spinlock was explicitly entered
	unsigned long imp_enabled;	//this spinlock was not entered, but a higher level was entered first
	unsigned long delays;				//number of times this spinlock spun
};

void enter_spinlock(unsigned int which);
//enters a spinlock level, if rules allow
void leave_spinlock(unsigned int which);
//leaves a spinlock level
void setup_spinlock_data();
///sets up data structure for spinlocks to work
void initialize_spinlock();
//must be called before interrupts and IRQ's are enabled
extern unsigned long test_and_set (unsigned long new_value, unsigned long *lock_pointer);
//use this to set an (unsigned int) that is shared
