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
}

////////////////////////
//ATA / ATAPI-4 driver//
////////////////////////

#define IDE_PRIMARY 		0x1F0
#define IDE_SECONDARY 	0x170

//these use the first base (IDE_PRIMARY and IDE_SECONDARY)
#define DATA_REG			 	0x0	//rw
	//data register

#define ERROR_REG			 	0x1	//read
	//error register

#define	FEATURE_REG			0x1	//write
	//feature register

#define SEC_CNT_REG			0x2 //rw
	//sector count register

#define LBA_LOW_REG			0x3	//rw
	//LBA low register

#define LBA_MID_REG			0x4	//rw
	//LBA mid register

#define LBA_HI_REG			0x5	//rw
	//LBA high register

#define DRV_HD_REG			0x6	//rw
	//drive / head register

#define STAT_REG				0x7	//read
	//status register

#define COM_REG					0x7	//write
	//command register 1 byte - if you read from this address you will actuall read the status register
	//only write this when BSY and DRQ and DMACK- are clear (except for device reset command)
	//invalid for a sleeping device
	//if written when BSY or DRQ are set then results are unknown except for device reset command

#define ALT_STAT_REG		0x206	//read
	//alternate status register

#define DEV_CTR_REG			0x206	//write
	//device control register

//execute a drive diagnostic
//this function executes on both master and slave drives on a given IDE controller
unsigned int execute_drive_diagnostic(unsigned int base)
{	//send the command byte to the appropriate IDE controller
	outportb(0x90, base + DEV_CTR_REG);
	//wait for the command to finish
	//wait_no_busy(base);
	//return the results from the command
	return inportb(base + ERROR_REG);
}

//create a function to set the power mode

//reset the disks (page 231 - D1153R18-ATA-ATAPI-4.pdf)
//FIRST: the BSY flag for both drives must not be set
		//SRST must be clear for >=5 us, then active for >= 5 us, then it must be cleared
//identify device and/or identify packet device is next after a software reset to determine what features are supported


//page 276 single device configurations


void examine_ide()
{	//examines IDE controllers for any drives present
	//check to see if the first IDE controller found
	unsigned int temp;
	unsigned int flags = 0;	//available drive information is stored here
	outportb(0x88, IDE_PRIMARY + LBA_LOW_REG);
	if (inportb(IDE_PRIMARY + LBA_LOW_REG) == 0x88)
	{	//the controller found
		flags |= 1;	//set the primary controller bit
		//reset_ATA1_controller(IDE_PRIMARY);
		outportb(0xA0, IDE_PRIMARY + DRV_HD_REG); // use 0xB0 instead of 0xA0 to test the second drive on the controller
		Delay(4); // wait for a little bit
		if (inportb(IDE_PRIMARY + STAT_REG) & 0x40) // see if the busy bit is set
		{	//issue the identify drive command and display the results (class 1 command)
			flags |= 2;	//set the primary master bit
		}
		outportb(0xB0, IDE_PRIMARY + DRV_HD_REG); // use 0xB0 instead of 0xA0 to test the second drive on the controller
		Delay(4); // wait for a little bit
		if (inportb(IDE_PRIMARY + STAT_REG) & 0x40) // see if the busy bit is set
		{
			flags |= 4;	//set the primary slave bit
		}
	}
	outportb(0x88, IDE_SECONDARY + LBA_LOW_REG);
	if (inportb(IDE_SECONDARY + LBA_LOW_REG) == 0x88)
	{	//the controller found
		flags |= 8;	//set the secondary controller bit
		//reset_ATA1_controller(IDE_SECONDARY);
		outportb(0xA0, IDE_SECONDARY + DRV_HD_REG); // use 0xB0 instead of 0xA0 to test the second drive on the controller
		Delay(4); // wait for a little bit
		if (inportb(IDE_SECONDARY + STAT_REG) & 0x40) // see if the busy bit is set
		{
			flags |= 0x10;	//set the secondary master bit
		}
		outportb(0xB0, IDE_SECONDARY + DRV_HD_REG); // use 0xB0 instead of 0xA0 to test the second drive on the controller
		Delay(4); // wait for a little bit
		if (inportb(IDE_SECONDARY + STAT_REG) & 0x40) // see if the busy bit is set
		{
			flags |= 0x20;	//set the secondary slave bit
		}
	}
	if (flags & 0x01)
	{	//only do checking if the PRIMARY IDE controller is found
		display("\t\tDrive diagnostic primary IDE controller\n");
		temp = execute_drive_diagnostic(IDE_PRIMARY);
		display("\t\tDrive diagnostic for PRIMARY returns: ");
		PrintNumber(temp);
		display("\n");
		if (flags & 0x02)
		{
			display("\t\tPrimary master found\n");
		}
		if (flags & 0x04)
		{
			display("\t\tPrimary slave found\n");
		}
	}
	if (flags & 0x08)
	{	//only do checking if the SECONDARY IDE controller is found
		display("\t\tDrive diagnostic secondary IDE controller\n");
		temp = execute_drive_diagnostic(IDE_SECONDARY);
		display("\t\tDrive diagnostic for SECONDARY returns: ");
		PrintNumber(temp);
		display("\n");
		if (flags & 0x10)
		{
			display("\t\tSecondary master found\n");
		}
		if (flags & 0x20)
		{
			display("\t\tSecondary slave found\n");
		}
	}
}
