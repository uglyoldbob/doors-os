#include "floppy.h"
#include "memory.h"
#include "dma.h"
#include "video.h"
#include "entrance.h"

extern "C" volatile unsigned int inportb(unsigned int port);		//entrance.asm
extern "C" unsigned int outportb(unsigned int value, unsigned int port);	//entrance.asm
extern "C" int WaitFloppyInt();	//entrance.asm

extern "C" unsigned int timer;	//entrance.asm

	//this is used so that a specialized memory allocater will not be required
	//but it is required that this buffer be allocated early so that it can claim a spot in lower memory

	//waits for the floppy drive to signal with an interrupt
//actually only the bottom two bytes of port and the bottom byte of the return value is used

//the base IO address for floppy drive communication
#define FLOPPY_PRIMARY_BASE     0x03F0
#define FLOPPY_SECONDARY_BASE   0x0370
//#define FLOPPY_SECONDARY_BASE		0x03F0
//according to ralf brown's port list, port 0x377 is shared between the secondary floppy controller and the second fixed disk controller

//offsets for the various registers
#define STATUS_REG_A            0x0000 /*PS2 SYSTEMS*/
#define STATUS_REG_B            0x0001 /*PS2 SYSTEMS*/
#define DIGITAL_OUTPUT_REG      0x0002
#define TAPE_DRIVE_REGISTER     0x0003
#define MAIN_STATUS_REG         0x0004
#define DATA_RATE_SELECT_REG    0x0004 /*PS2 SYSTEMS*/
#define DATA_REGISTER           0x0005
#define DIGITAL_INPUT_REG       0x0007 /*AT SYSTEMS*/
#define CONFIG_CONTROL_REG      0x0007 /*AT SYSTEMS*/
#define PRIMARY_RESULT_STATUS   0x0000
#define SECONDARY_RESULT_STATUS 0x0000

//command bytes for the floppy disk controller
#define FIX_DRIVE_DATA          0x03
#define SPECIFY									0x03
#define CHECK_DRIVE_STATUS      0x04
#define RECALIBRATE_DRIVE       0x07
#define CHECK_INTERRUPT_STATUS  0x08
#define FORMAT_TRACK            0x4D
#define READ_SECTOR             0x66
#define READ_DELETE_SECTOR      0xCC
#define READ_SECTOR_ID          0x4A
#define READ_TRACK              0x42
#define SEEK_TRACK              0x0F
#define WRITE_SECTOR            0xC5
#define WRITE_DELETE_SECTOR     0xC9

#define DISK_PARAMETER_ADDRESS 0x000FEFC7 /* location where disk parameters */
                                          /* is stored by bios */

//c++
floppy::floppy()
{	//since kmalloc is used in this function, it is advised to create an instance of this class after the 
		//kmalloc routine has been initialized
	//don't forget that structures have been modified so that more than one floppy drive can be accessed
		//by the same class
	//this driver as of right now checks for drives (0-3) on primary and secondary floppy controllers (8 total)
	unsigned int check = 0;
	unsigned int length = 0;
	unsigned int counter;
	unsigned int error;
	unsigned int num_floppy_drives = 0;
	floppy_information *temp_floppy;
	temp_floppy = new floppy_information[8];
	floppies = temp_floppy;	//point to to the newly created structure
	for (counter = 0; counter< 4; counter++)
	{
		temp_floppy[(counter * 2)].drive_number = 0;
		temp_floppy[(counter * 2)].floppy_number = counter;
		temp_floppy[(counter * 2)].base = FLOPPY_PRIMARY_BASE;
		memcopy(&((temp_floppy[(counter * 2)].floppy_disk)), (unsigned char *)DISK_PARAMETER_ADDRESS, sizeof(floppy_parameters));
		temp_floppy[(counter * 2) + 1].drive_number = 0;
		temp_floppy[(counter * 2) + 1].floppy_number = counter;
		temp_floppy[(counter * 2) + 1].base = FLOPPY_SECONDARY_BASE;
		memcopy(&((temp_floppy[(counter * 2) + 1].floppy_disk)), (unsigned char *)DISK_PARAMETER_ADDRESS, sizeof(floppy_parameters));

		error = reset_floppy((counter * 2));
		if (error == 0)
		{
			num_floppy_drives++;
			temp_floppy[(counter * 2)].drive_number = get_drive_num();
			display("Floppy drive ");
			PrintNumber(temp_floppy[(counter * 2)].drive_number);
			display(" is present on primary controller\n");
			outportb(0x8, FLOPPY_PRIMARY_BASE + DIGITAL_OUTPUT_REG);	//disable drive motor
		}
		else
		{
			temp_floppy[(counter * 2)].drive_number = INVALID_DRIVE;
			/*display("Floppy drive ");
			PrintNumber(counter * 2);
			display(" is not present with return code: ");
			PrintNumber(error);
			display("\n");*/
		}
		error = reset_floppy((counter * 2) + 1);
		if (error == 0)
		{
			num_floppy_drives++;
			temp_floppy[(counter * 2) + 1].drive_number = get_drive_num();
			display("Floppy drive numbered ");
			PrintNumber(temp_floppy[(counter * 2) + 1].drive_number);
			display(" is present on secondary controller\n");
			outportb(0x8, FLOPPY_SECONDARY_BASE + DIGITAL_OUTPUT_REG);	//disable drive motor
		}
		else
		{
			temp_floppy[(counter * 2) + 1].drive_number = INVALID_DRIVE;
			/*display("Floppy drive ");
			PrintNumber((counter * 2) + 1);
			display(" is not present with return code: ");
			PrintNumber(error);
			display("\n");*/
		}
	}
	//allocate for the proper number of floppy drives detected
	if (num_floppy_drives == 1)
		floppies = new floppy_information[num_floppy_drives + 1];	
	else
		floppies = new floppy_information[num_floppy_drives];	
	unsigned int current_floppy = 0;
	for (counter = 0; current_floppy < num_floppy_drives; counter++)
	{
		if (temp_floppy[counter].drive_number != INVALID_DRIVE)
		{
			floppies[current_floppy].floppy_disk = temp_floppy[counter].floppy_disk;
			floppies[current_floppy].base = temp_floppy[counter].base;
			floppies[current_floppy].drive_number = temp_floppy[counter].drive_number;
			floppies[current_floppy].floppy_number = temp_floppy[counter].floppy_number;
			floppies[current_floppy].drive_identifier = 0;	//TODO: replace with a drive distinguiser pattern
			floppies[current_floppy].number_sides = 2;	//TODO: detect the number of sides for the current disk
				//INVALID_DRIVE means no disk
			current_floppy++;
		}
	}
	//deallocate the memory acquired for the temporary floppy disk information buffers
	delete[] temp_floppy;
	//show how many floppy drives were detected
	num_drives = num_floppy_drives;
	display("Detected ");
	PrintNumber(num_floppy_drives);
	display(" floppy drives\n");
	//attempt to retrieve information for each of the floppy drives detected
		//looking at the bios data area is probably not the best way
		//this will probably have to be done for each floppy disk stuck into the floppy drive anyways
	
		//floppy_disk array will have to be initialized after detecting how many floppy drives are present
	//initialize the floppy disk information structure
//	reset_floppy(base, 0);
//	outportb(0, base + DIGITAL_OUTPUT_REG);
	//determine how large a sector is and allocate space for one sector
	drive_name = 0;
	//this should loop for each floppy drive detected
		//and create a seperate buffer for each, or find the largest buffer required between all of the detected floppy drives
	for (counter = 0; counter < floppies[0].floppy_disk.bytes_per_sector; counter++)
		length *= 2;
	length *= 128;
	sector_buffer = (unsigned long *)kmalloc(0x1000);
}

floppy::~floppy()
{
	kfree(sector_buffer);
	delete[] floppies;
}

int floppy::number_drives()
{
	return num_drives;
}

int floppy::get_drive_number(int drive_order)
{
	if ((drive_order >= 0) && (drive_order < num_drives))
	{
		return floppies[drive_order].drive_number;
	}
	else
	{
		return INVALID_DRIVE;
	}
}

int floppy::set_drive(unsigned int drive)
{	//this checks the array of disk data and finds which record it is
	for (int counter = 0; counter < num_drives; counter++)
	{
		if (floppies[counter].drive_number == drive)
		{
			drive_order = counter;
			return 0;
		}
	}
	return -1;	//indicate that the drive number is not accessible with this driver
}

int floppy::wait_to_recieve(unsigned int drive_order)
{
	//while ((inportb(base + CHECK_DRIVE_STATUS) & 0xC0) != 0xC0){};
	unsigned int temp;
	while (1)
	{
		temp = inportb(floppies[drive_order].base + CHECK_DRIVE_STATUS);
		if ((temp & 0xC0) == 0xC0)	//only let it retrieve data while the controller is not busy?
		{
			return 0;
		}
		else if ((temp & 0xC0) == 0x80)
		{
			display("Controller is waiting for data, this is unexpected, MSR:");
			PrintNumber(temp);
			display("\n");
		}
		Delay(1000);
	}
	return 0;
}

void floppy::send_byte(unsigned int drive_order, unsigned char command)
{	//this actually sends a byte
	unsigned int temp;
	while (1)
	{
		temp = 0;
		temp = inportb(floppies[drive_order].base + CHECK_DRIVE_STATUS);
		if ((temp & 0xC0) == 0xC0)
		{
			display("Controller wants to give, this is unexpected, MSR:");
			PrintNumber(temp);
			display("\n");
		}
		else if ((temp & 0xC0) == 0x80)
		{
			outportb(command, floppies[drive_order].base + DATA_REGISTER);
			return;
		}
		Delay(1000);
	}
}

void floppy::check_floppy_status(unsigned int drive_order, unsigned int *st0, unsigned int *cylinder)
{	//performs the FDC instruction and returns all applicable results
	//waitSendFloppy(base);
	send_byte(drive_order, CHECK_INTERRUPT_STATUS);
	wait_to_recieve(drive_order);
	*st0 = inportb(floppies[drive_order].base + DATA_REGISTER);
	wait_to_recieve(drive_order);
	*cylinder = inportb(floppies[drive_order].base + DATA_REGISTER);
}

void floppy::specify_drive(unsigned int drive_order)
{	//this is the specify command
	send_byte(drive_order ,FIX_DRIVE_DATA);/*config/specify command*/
	send_byte(drive_order ,floppies[drive_order].floppy_disk.steprate_headunload);
	send_byte(drive_order ,floppies[drive_order].floppy_disk.headload_ndma);	//set bit 0 for nondma transfer, clear it for DMA transfer
		//no results
}

int floppy::recalibrate_drive(unsigned int drive_order)
{	//drive = 0,1,2,3
	//this is the recalibrate drive command
	//recalibrate drive to detect the presence of a drive
	//recalibrate, trying fastest settings and downgrading as they fail
		//check for success with read id
	unsigned int st0, cylinder;
	select_drive(drive_order);
	//make sure motor is turned on
	//display("Issuing calibrate drive command.\n");
	do
	{
		send_byte(drive_order , RECALIBRATE_DRIVE); /*Calibrate drive*/
  	send_byte(drive_order , floppies[drive_order].floppy_number);
		if (WaitFloppyInt() == -1)
			return -1;
  	check_floppy_status(drive_order, &st0, &cylinder); /*check interrupt status and
                                                store results in variables
                                                st0 and cylinder*/
		if ((st0  & 0xFC) != 0x20) 
		{	//the last 2 bits are used to report the currently selected drive
			display("Error in calibrate drive, st0:");
			PrintNumber(st0);
			display("\n");
			return -1;
		}
		//display("Still issuing calibrate command\n");
	} while (cylinder != 0);
	//repeat until the floppy drive is over cylinder 0
	//if it made it this far, then the drive "should" be present
	//display("Finished with calibrate drive command\n");
	return 0;
}

int floppy::seek_to_cylinder(unsigned int cylinder, unsigned int head, unsigned int drive_order)
{
	unsigned int st0, cylinder_check;
	do
	{
		send_byte(drive_order, SEEK_TRACK);
		send_byte(drive_order, head<<2 | floppies[drive_order].floppy_number);
		send_byte(drive_order, cylinder);
		if (WaitFloppyInt() == -1)
			return -1;	//wait for the impending interrupt or a timeout
		check_floppy_status(drive_order, &st0, &cylinder_check);
	} while (cylinder_check != cylinder);

	return 0;
}

int floppy::ccr(unsigned int drive_order)
{	//enables the drive/disk combo for the fastest functioning speed
	//3, 0, 1, 2 are the speed numbers from fastest to slowest
	//hopefully nobody will call this when the drive is not enabled
		//TODO: check to make sure the drive is enabled and ready?
	if (recalibrate_drive(drive_order) == -1)
		return -1;	//spot errors and pass them on down the line
	outportb(3, floppies[drive_order].base + CONFIG_CONTROL_REG);
	//read id command now	
	if (read_id(drive_order) == -1)
		return -1;
}

int floppy::reset_floppy(unsigned int drive_order)
{
	unsigned int st0, cylinder;	//this will be used for any necessary storage of FDC states
	//reset the floppy disk to a known state
	outportb(0x00, floppies[drive_order].base + DIGITAL_OUTPUT_REG);
	Delay(100);
	outportb(0x0C, floppies[drive_order].base + DIGITAL_OUTPUT_REG);
	//set speed for the drive
	outportb(0, floppies[drive_order].base + CONFIG_CONTROL_REG);
	if (WaitFloppyInt() == -1)
		return -1;
	check_floppy_status(drive_order, &st0, &cylinder);
	check_floppy_status(drive_order, &st0, &cylinder);
	check_floppy_status(drive_order, &st0, &cylinder);
	check_floppy_status(drive_order, &st0, &cylinder);
	specify_drive(drive_order);
	if (recalibrate_drive(drive_order) == -1)
		return -1;
	return 0;
}

void floppy::getResults(unsigned int *st0, unsigned int *st1, unsigned int *st2, unsigned int *cylinder_r, 
					unsigned int *head_r, unsigned int *sector_r, unsigned int *size_r, unsigned int drive_order)
{	//recieves the results from the floppy drive after a sector/track read/write command
	wait_to_recieve(drive_order);
	*st0 = inportb(floppies[drive_order].base + DATA_REGISTER);
	wait_to_recieve(drive_order);
	*st1 = inportb(floppies[drive_order].base + DATA_REGISTER);
	wait_to_recieve(drive_order);
	*st2 = inportb(floppies[drive_order].base + DATA_REGISTER);
	wait_to_recieve(drive_order);
	*cylinder_r = inportb(floppies[drive_order].base + DATA_REGISTER);
	wait_to_recieve(drive_order);
	*head_r = inportb(floppies[drive_order].base + DATA_REGISTER);
	wait_to_recieve(drive_order);
	*sector_r = inportb(floppies[drive_order].base + DATA_REGISTER);
	wait_to_recieve(drive_order);
	*size_r = inportb(floppies[drive_order].base + DATA_REGISTER);
}

int floppy::read_sector(unsigned int drive_num, unsigned long sector_number, unsigned int *buffer)
{	//starts at sector 0 for sector_number
	//this should be modified to use the same buffer every time
	//and copy the data to the requested buffer space
	//drive = 00, 01, 02, 03
	unsigned int st0, st1, st2, cylinder_r, head_r, sector_r, size_r;//, cylinder_check;
	//these store the results from the read command
	unsigned int length = 1;
	unsigned int counter = 0;
	unsigned int command;
	unsigned int cylinder, head, sector;
	if (set_drive(drive_num) == -1)
	{	//retrieve the array number that contains this drive number
		//check to make sure that the drive number requested is actually in the list of drives controlled
		display("No such floppy disk numbered: ");
		PrintNumber(drive_num);
		display("\n");
		return -1;
	}
	//sector_number--;
	sector = (sector_number % floppies[drive_order].floppy_disk.sectors_per_track) + 1;
	cylinder = (sector_number / floppies[drive_order].floppy_disk.sectors_per_track) / 
		floppies[drive_order].number_sides;	//sectornum / (sectors per track * number sides)
	head = (sector_number / floppies[drive_order].floppy_disk.sectors_per_track) % 
		floppies[drive_order].number_sides;			//2 heads on a floppy drive
	for (counter = 0; counter < floppies[drive_order].floppy_disk.bytes_per_sector; counter++)
		length *= 2;
	length *= 128;
	//sector_size = length;
		//size of the sector in bytes
	//set floppy disk status to known state
	if (reset_floppy(drive_order) == -1)
		return -1;
	if (ccr(drive_order) == -1)
		return -1;
	//configure the DMA (channel 2)
	startDMA((unsigned int)sector_buffer, length - 1, 2, 0x46);	
		//TODO: verify the code for this (0x45 is what it was)
		//i think this might be using channel 3 by mistake
	//give the seek track command
	seek_to_cylinder(cylinder, head, drive_order);
	//the length of time in milliseconds it takes for the head to settle after moving
	Delay(floppies[drive_order].floppy_disk.head_settle_time);
	send_byte(drive_order, READ_SECTOR);
	send_byte(drive_order, head<<2 | floppies[drive_order].floppy_number);
	send_byte(drive_order, cylinder);
	send_byte(drive_order, head);
	send_byte(drive_order, sector);	//TODO: find size of a sector properly instead of assuming 512 bytes
	send_byte(drive_order, floppies[drive_order].floppy_disk.bytes_per_sector);  /*sector size = 128*2^size*/
	send_byte(drive_order, floppies[drive_order].floppy_disk.sectors_per_track); /*last sector*/
	send_byte(drive_order, floppies[drive_order].floppy_disk.gap_length);        /*27 default gap3 value*/
	send_byte(drive_order, floppies[drive_order].floppy_disk.data_length);       /*default value for data length*/
	if (WaitFloppyInt() == -1)
			return -1;	//wait for the completion of the command
	Delay(floppies[drive_order].floppy_disk.steprate_headunload & 0x0F);
	getResults(&st0, &st1, &st2, &cylinder_r, &head_r, &sector_r, &size_r, drive_order);
	memcopy((void *)buffer, (void *)sector_buffer, length);
	//turn off floppy disk motor
	outportb(0x8, floppies[drive_order].base + DIGITAL_OUTPUT_REG);
	return 0;	//indicate success
}

//TODO: debug the write sector command
//make sure that the byte changed will not make the disk unbootable
	//load the bootsector
	//change a byte
	//write the sector
	//read the sector
	//verify the changed sector
int floppy::write_sector(unsigned int drive_num, unsigned long sector_number, unsigned int *buffer)
{	//starts at sector 0 for sector_number
	//this should be modified to use the same buffer every time
	//and copy the data to the requested buffer space
	//drive = 00, 01, 02, 03
	unsigned int st0, st1, st2, cylinder_r, head_r, sector_r, size_r;//, cylinder_check;
	//these store the results from the write command
	unsigned int length = 1;
	unsigned int counter = 0;
	unsigned int command;
	unsigned int cylinder, head, sector;
	if (set_drive(drive_num) == -1)
	{	//retrieve the array number that contains this drive number
		//check to make sure that the drive number requested is actually in the list of drives controlled
		display("No such floppy disk numbered: ");
		PrintNumber(drive_num);
		display("\n");
		return -1;
	}
	sector_number--;	//so that the first sector of the disk will be numbered 1
	sector = (sector_number % floppies[drive_order].floppy_disk.sectors_per_track) + 1;
	cylinder = (sector_number / floppies[drive_order].floppy_disk.sectors_per_track) / 
		floppies[drive_order].number_sides;	//sectornum / (sectors per track * number sides)
	head = (sector_number / floppies[drive_order].floppy_disk.sectors_per_track) % 
		floppies[drive_order].number_sides;			//2 heads on a floppy drive
	for (counter = 0; counter < floppies[drive_order].floppy_disk.bytes_per_sector; counter++)
		length *= 2;
	length *= 128;
	memcopy((void *)sector_buffer, (void *)buffer, length);
	//sector_size = length;
		//size of the sector in bytes
	//set floppy disk status to known state
	if (reset_floppy(drive_order) == -1)
		return -1;
	if (ccr(drive_order) == -1)
		return -1;
	//configure the DMA (channel 2)
	startDMA((unsigned int)sector_buffer, length - 1, 2, 0x4A);
		//this should be the proper dma mode to put data into the device jus the same as it comes out
		//TODO: debug and make sure this works
	//give the seek track command
	seek_to_cylinder(cylinder, head, drive_order);
	//the length of time in milliseconds it takes for the head to settle after moving
	Delay(floppies[drive_order].floppy_disk.head_settle_time);
	send_byte(drive_order, WRITE_SECTOR);
	send_byte(drive_order, head<<2 | floppies[drive_order].floppy_number);
	send_byte(drive_order, cylinder);
	send_byte(drive_order, head);
	send_byte(drive_order, sector);	//TODO: find size of a sector properly instead of assuming 512 bytes
	send_byte(drive_order, floppies[drive_order].floppy_disk.bytes_per_sector);  /*sector size = 128*2^size*/
	send_byte(drive_order, floppies[drive_order].floppy_disk.sectors_per_track); /*last sector*/
	send_byte(drive_order, floppies[drive_order].floppy_disk.gap_length);        /*27 default gap3 value*/
	send_byte(drive_order, floppies[drive_order].floppy_disk.data_length);       /*default value for data length*/
	if (WaitFloppyInt() == -1)
			return -1;	//wait for the completion of the command
	Delay(floppies[drive_order].floppy_disk.steprate_headunload & 0x0F);
	getResults(&st0, &st1, &st2, &cylinder_r, &head_r, &sector_r, &size_r, drive_order);
	//turn off floppy disk motor
	outportb(0x8, floppies[drive_order].base + DIGITAL_OUTPUT_REG);
	return 0;	//indicate success
}


int floppy::bytes_per_sector(unsigned int drive_num)
{
	set_drive(drive_num);
	int length = 1;
	for (int counter = 0; counter < floppies[drive_order].floppy_disk.bytes_per_sector; counter++)
		length *= 2;
	length *= 128;
	if (length == 0)
		return -1;
	return length;
}

int floppy::read_id(unsigned int drive_order)
{
	//display("Read id\n");
	unsigned int st0, st1, st2, cylinder, head, sector, size;
	send_byte(drive_order, READ_SECTOR_ID);
	send_byte(drive_order, floppies[drive_order].floppy_number);
	if (WaitFloppyInt() == -1)
		return -1;	//error catching
	getResults(&st0, &st1, &st2, &cylinder, &head, &sector, &size, drive_order);
	//set sector size base off of the results from the getResults function
	floppies[drive_order].floppy_disk.bytes_per_sector = size;
	//display("Bytes per sector code: ");
	//PrintNumber(size);
 	//display("\n");
	if ((st0 & 0xC0) == 0x40)
	{
		if ((st1 & 0x01) == 0x01)
		{
			display("Error in read id\n");
		}
	}
	if ((st1 & 0x04) == 0x04)
	{
		display("Cannot read ID field without error\n");
	}
	return 0;
}

int floppy::select_drive(unsigned int drive_order)
{	//enables the floppy drive motor given a drive_order to operate on
	unsigned int command;
	command = (0x10<<(floppies[drive_order].floppy_number));
	command += 0x0C + floppies[drive_order].floppy_number;
	outportb(command, floppies[drive_order].base + DIGITAL_OUTPUT_REG);
	return 0;
}

char * floppy::identify_driver()
{	//not implemented right now, but will be later on
	return 0;
}

//add functions to read files from the root directory of the disk
//then multitasking
//virtual memory
//complete keyboard driver
//...
//write functions to write to disk (carefully)
