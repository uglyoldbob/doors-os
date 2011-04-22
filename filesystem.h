//filesystem.h
#ifndef _FILESYSTEM_H_
#define _FILESYSTEM_H_
#include "disk.h"
#include "file.h"
#include "string.h"

struct krnl_FILE;

//structure for filesystem independent directory item
	//name (char *)
	//extension (char *)	
	//size (unsigned long)
	//permissions (unsigned long)
	//date modified		//12 bits for the year, 4 bits for the month, 5 bits for the day, 
				//5 bits for the hour, 6 bits for the minute, 6 bits for the seconds
				//38 bits total
	//date accessed
	//owner

#define DIR_ITEM_FILE 1
#define DIR_ITEM_FOLD 2
#define DIR_ITEM_INVA 3

struct dir_item
{	//this is used when listing the contents of a directory
	//TODO: create flag definitions for the permissions and type fields
	char *name;
	char *extension;
	unsigned long size;
	unsigned long permissions;
	unsigned char type;
	unsigned short date_mod1;	//year and month 12 year, 4 month
	unsigned long date_mod2;	//bit format - xxxxxxdddddhhhhhmmmmmmssssss
								//			 - 6 x   5 d  5 h  6 m   6 s
	unsigned short date_crt1;
	unsigned long date_crt2;
	unsigned short date_acc1;
	unsigned long date_acc2;
	unsigned long owner;	//not supported yet, but it's there
	unsigned long anything1;	//this is to be used by the filesystem for anything it feels like
};

//this structure holds vital information
	//it is required to do any work with files
struct FILE_INFO
{
	char *filename;
	unsigned char permissions;
	unsigned long offset;	//offset into the file
	unsigned long length;	//total length of the file
	unsigned int *buffer;	//a buffer to hold a segment of the file
	unsigned int buffer_length;	//length of the buffer
	unsigned int buffer_offset;	//offset of the beginning of the buffer into the file
	void *fs_spec;	//a filesystem specific pointer regarding the file data
};

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
		char *get_current_directory();			//returns the current location the filesystem is looking at
		virtual dir_item *list_contents();	
			//lists the contents of the current directory, places results into a structure array
		virtual int enter_directory(const char* dir);
			//moves over to an adjacently visible directory
		virtual krnl_FILE *open_file(char *filename);
			//opens a file and returns a descriptor for that file
		virtual unsigned int eof(krnl_FILE *descriptor);
		virtual unsigned char get_b(krnl_FILE *descriptor, filesystem *owner);
		virtual unsigned short get_w(krnl_FILE *descriptor, filesystem *owner);
		virtual unsigned long get_dw(krnl_FILE *descriptor, filesystem *owner);
		virtual unsigned long seek(krnl_FILE *descriptor, unsigned long position, filesystem *owner);
		virtual unsigned int get_buffer_size();
			//returns the size of the block that file data is passed in
		virtual unsigned char *get_buffer(unsigned int offset, krnl_FILE *file);
			//returns the buffer that contains the proper offset into the file
	protected:
		char *current_directory;	//stores the current directory that this filesystem will look at
		virtual int is_mounted();	//used to determine if the filesystem is currently mounted
			//-1 is not mounted, 1 is mounted
		//the subclasses will store any information that is needed to access that directory
};
#endif

//list_directory contents
//change current directory
//(variable to store current directory)
//load file to memory
//get file flags

//a terminal should be treated like a file and thus eliminating the need for the terminal class (maybe)

//retrieve folder listing
//get file/folder permissions
//go to another level of folder
//

//list folder contents
	//return a char* with a /n between each entry
	//return a char ** array of all the entries
	//return a structure array containing the required information
