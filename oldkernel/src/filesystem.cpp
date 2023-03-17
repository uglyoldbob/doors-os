//filesystem.cpp
#include "filesystem.h"
#include "string.h"

filesystem::filesystem()
{
}

filesystem::~filesystem()
{
}

int filesystem::mount(disk *somewhere, unsigned int drive_num)
{
	return -1;
}

int filesystem::unmount(disk *somewhere, unsigned int drive_num)
{
	return -1;
}

int filesystem::is_mounted()
{
	return -1;
}

char * filesystem::get_current_directory()
{
	char *return_value;
	if (is_mounted() == 1)
	{	
		return_value = new char [strlen(current_directory) + 1];
		strcpy(return_value, current_directory);
		return return_value;
	}
	else
	{
		return 0;
	}
}

dir_item *filesystem::list_contents()
{
	return 0;
}

int filesystem::enter_directory(const char* dir)
{
	return -1;
}

krnl_FILE *filesystem::open_file(char *filename)
{
	return 0;
}

unsigned int filesystem::eof(krnl_FILE *descriptor)
{
	return 0;
}

unsigned long filesystem::seek(krnl_FILE *descriptor, unsigned long position, filesystem *owner)
{
	return -1;
}

unsigned char filesystem::get_b(krnl_FILE *descriptor, filesystem *owner)
{
	return 0;
}

unsigned short filesystem::get_w(krnl_FILE *descriptor, filesystem *owner)
{
	return 0;
}

unsigned long filesystem::get_dw(krnl_FILE *descriptor, filesystem *owner)
{
	return 0;
}

unsigned int filesystem::get_buffer_size()
{	//returns the size of the block that file data is passed in
	return 0;
}

unsigned char *filesystem::get_buffer(unsigned int offset, krnl_FILE *file)
{	//returns the buffer that contains the proper offset into the file
	return 0;
}

int open(unsigned char *filename, unsigned int flags)
{
	return -1;
}

int close(unsigned char *filename, unsigned int flags)
{
	return -1;
}
