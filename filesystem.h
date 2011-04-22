//filesystem.h
#include "disk.h"
#ifndef _FILESYSTEM_H_
#define _FILESYSTEM_H_
class filesystem
{	//storage involving this class will involve an array
	//initializers will setup the minimum of a "this filesystem doesn't work" setup
		//returns errors when access attempts are made
	//there will be specific mount and unmount commands that will initialize the filesystem data (according to the disk that it is assigned to)
	//the driver will be required to keep a copy of a disk structure and a drive number in order to function properly
	//in this manner, any filesystem "should" be able to access file storage on "ANY" medium (to include ramdrives, network driver, et cetera)
	public:
		filesystem();
		virtual ~filesystem();
		virtual int mount(disk *, unsigned int);	//uses the external disk reference number
		virtual int unmount(disk *, unsigned int);	//uses the external disk reference number
	protected:
	private:
};
#endif

//list_directory contents
//change current directory
//(variable to store current directory)
//load file to memory
//get file flags
