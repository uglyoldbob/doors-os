#include "disk.h"
#include "interrupt_table.h"
#include "floppy.h"
#include "memory.h"
#include "entrance.h"
#include "video.h"
#include "floppy.h"

//this should be restructured for C++

//file naming convention
/* /(drive name)/ ex floppy1 floppy2 cd1 cd2 cd3
	for hard drives, the second number specifies which partition is being accessed on that particular hard drive
	hd1/1/ hd1/2/ hd2/1/
/	/floppya/boot/grub/
	/floppya/kernel.bin
	/
*/

//process filename, return (drive and partition number information)
//figure out which drive access code is required from the drive number
	//access the partition and hopefully we have a filesystem driver for it
	//and hopefully the partition is formatted
//lookup table for drives
	//drive name (floppy0, floppy1, hd1, cd3, rd1)
	//drive number

/*The kernel and disk driver will keep copies of disk structure information
	The filesystem driver will be run by the kernel, so there will be no need for the filesystem driver to keep that information
	It can keep a copy of the information, but I don't see any reason for it at the moment*/

//general structure of the disk driver (not the filesystem driver)
	//initialize function (this function is responsible for initializing and detecting drives
		//this should allow support for unsupported storage devices to be easily added
		//this function should return an array of devices detected by the driver
	//function selector: returns a pointer for each various functions of the driver
		//ex: function_pointer get_function(DISK_READ_SECTOR);
	//common functions used by filesystem drivers (read sector(s), write sector(s))
		//the driver does not necessarily have to give the command to the drive that the function is named
		//the floppy driver can issue a read track command instead of the read sectors command (since there is no read sectors command that I am aware of)
		//the function names have to be present, but can return an error if the device does not support that feature
			//cd drivers probably will not support write functions

//structure of filesystem driver
	//if the operating system is expected to boot from a filesystem it is recommended that a small read only driver be compiled into the kernel

disk::disk()
{
	drive_name = 0;	//null terminated string
	current_unused_drive_num = 0;
}

disk::~disk()
{
}

int disk::number_drives()
{
	return 0;
}

int disk::get_drive_number(int drive_order)
{
	return INVALID_DRIVE;
}

unsigned int disk::get_drive_num()	//returns a usable drive number
{
	display("get_drive_num()\n");
	current_unused_drive_num++;
	return (current_unused_drive_num - 1);
}

int disk::read_sector(unsigned int drive_num, unsigned long sector_number, unsigned int *buffer)
{
	return -1;
}

int disk::write_sector(unsigned int drive_num, unsigned long sector_number, unsigned int *buffer)
{
	return -1;
}

int disk::bytes_per_sector(unsigned int drive_num)
{
	return -1;
}

char *disk::identify_driver()	//returns a string identifying the driver
{
	return 0;
}
/*
struct sectorReturn readSector(unsigned char driveNum, unsigned long sectorNumber)
{	//general purpose read sector
	struct sectorReturn sector;
	sector.size = 0x1000;		//TODO: need a function to find out the correct sector size
	sector.data = (unsigned char *)malloc(sector.size);
	if (driveNum < 4)
	{	//floppy drive
		if (floppy_read_sector(sectorNumber, driveNum, (unsigned int)sector.data, 0x3F0) == -1)
		{	//error occurred while reading, try other floppy controller?
			free(sector.data);
			sector.size = 0;	//this will indicate that an error happened
		}
		return sector;
	}
	free(sector.data);
	sector.size = 0;	//driveNum in unknown
	return sector;
}*/

