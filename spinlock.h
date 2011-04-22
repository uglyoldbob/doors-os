//most interruptable
#define SL_BLANK 0	//don't use
#define SL_MEM_MNG 1
#define SL_IRQ1 2
#define SL_MESSAGE 3	//message pumping system
//least interruptable
#define NUMBER_TYPES 4

struct SL_STATES
{
	unsigned int exp_enabled;		//this spinlock was explicitly entered
	unsigned int imp_enabled;	//this spinlock was not entered, but a higher level was entered first
	unsigned int delays;				//number of times this spinlock spun
};

void enter_spinlock(unsigned int which);
//enters a spinlock level, if rules allow
void leave_spinlock(unsigned int which);
//leaves a spinlock level
void setup_spinlock_data();
///sets up data structure for spinlocks to work
void initialize_spinlock();
//must be called before interrupts and IRQ's are enabled
extern unsigned int test_and_set (unsigned int new_value, unsigned int *lock_pointer);
//use this to set an (unsigned int) that is shared
