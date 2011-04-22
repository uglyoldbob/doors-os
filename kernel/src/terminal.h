//terminal.h
class terminal
{	//this class provides terminal support
	//this will interface with class video, serial, and whatever else the information for the terminal might be sent across
public:
	terminal();
	int output(unsigned char);	
		//send a byte to the terminal
		//terminal code should keep a buffer to process multi-byte commands
		//although I'm not sure how to figure out if a code is a single or multi byte command
	int input(unsigned char);	
		//receive a character from the terminal (wherever the terminal is configured to receive from)
	int possess(unsigned long);	
		//attempt to obtain ownership of the terminal, returns an error if ownership cannot be obtained
	int release(unsigned long);
		//attempt to release ownership of the terminal
private:
	unsigned long owner;	//a terminal needs an owner, most likely at most 1 owner, not positive on this yet
	
}

//terminal options (currently)
 //output
 //printer, serial, local display (can be more than one)
 
 //input
 //keyboard, serial

//possess and release need to make sure that the identifier given to them is allowed to perform
	//and also if the operation can be performed on that terminal at that point in time
    //there should be a double verification for the identifier so that apps do not cause problems with each other
	//the worst problem that should be encountered should be running out of available terminals

