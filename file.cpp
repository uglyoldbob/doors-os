#include "file.h"
#include "filesystem.h"

class filesystem;

//file::file()
//{	//initialize the class structure
//}

krnl_FILE *open(char *filename, unsigned int flags, filesystem *owner)
{	//this will allow data from a file to be read
	krnl_FILE *descriptor;
	descriptor = owner->open_file(filename);
	return descriptor;
	//TODO: add a temporary filesystem * to the list of arguments
		//until a proper table can be constructed
	//1. parse the filename and figure out where the file is located (filesystem class)
	//2. make sure the file is not currently locked out
		//a file can be locked only if the obtainee is the very first to try to get it
		//once a file has been opened (shared), anybody can open and close it
		//a read lock implies a write lock, a write lock does not have to include a read lock
		//when a write lock is obtained, anybody else trying to read from that file probably won't receive updates
		//when a file is opened for write permissions, a lock is automatically implied
			//a file cannot be opened for write permissions without a write lock
		//also ensure that the file and or media is not read only
	//3. fill out information in the filedescriptor given
		//after making sure that the file descriptor is null
	//4. figure out how large the file is and how much memory must be allocated
		//here is where some "paging" routines can be taken advantage of
		//i will figure out the virtual memory routine cheating later
	//5. allocate memory for the file in the appropriate manner
		//the memory allocated might need to have special flags set in the pdt and whatnot
			//depending on the access rights that are assigned to it
		
	//when memory is written to (when the file is modifiable), write the changes to disk
		//this should be an option that can be passed
		
}

unsigned int eof(krnl_FILE *descriptor, filesystem *owner)
{
	return owner->eof(descriptor);
}

unsigned long seek(krnl_FILE *descriptor, unsigned long position, filesystem *owner)
{
	return owner->seek(descriptor, position,owner);
}

unsigned char get_b(krnl_FILE *descriptor, filesystem *owner)
{
	return owner->get_b(descriptor, owner);
}

unsigned short get_w(krnl_FILE *descriptor, filesystem *owner)
{
	return owner->get_w(descriptor, owner);
}

unsigned long get_dw(krnl_FILE *descriptor, filesystem *owner)
{
	return owner->get_dw(descriptor, owner);
}

int close(unsigned char *filename, unsigned int flags, krnl_FILE *descriptor, filesystem *owner)
{
}

