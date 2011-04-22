//global variables that will define all of Doors' customizable settings
unsigned long VirMemSize = 0;
	//the number of pages that virtual memory composes
unsigned long KernelEnd = 0;
	//store the last memory address used by the kernel file (take size and add beginning)	

void GetSettings()
{	//sets settings to defaults, but will eventually read them from a file
	ReadSector(0x0500, 0, 0);	//0 = 'A'
	display("ASDF");
	VirMemSize = 0;
		//there is no code to actually implement virtual memory yet
	KernelEnd = 0x3000;
		//for now set this to 12KB, to be on the safe side (this will eventually be read from disc)
	return;
}
