#include "keyboard.h"
#include "message.h"
#include "entrance.h"
#include "video.h"

//unsigned int BootType;	//warm or cold boot; 0 = cold, 1 = warm
	//not valid on all computers

//the keyboard is initalized to use translated scancode set 2 (sources I have read say it is practically gauranteed to work on all keyboards (sets 1 and 3 might not work on all for some weird reason)
//the keyboard only sends codes one byte at a time, so each scancode is processed with a buffer
//when the last byte of a scancode is recieved, it is translated and placed in the system notification buffer
	//via a spinlock aware function


//00 means that that flag doesnt do anything
	//for caps lock it means shift does the same thing
	//for num lock it means it has no effect
	//for shift, it means no effect
	//a regular 0 means a multi-byte
EXTERNC const char ASCII_TRANSLATE[] = {
	//regular, shift, caps, numlock
	//for keys that have a regular of 27, shift refers to the multibyte number of the regular code, 
		//and numlock is another multibyte number (0 refers to the numlock having no effect)
	0, 0, 0, 0, //this entry is unused (keyboard chart starts at 1)
	27, 0, 0, 0,
	49, 33, 0, 0,
	50, 64, 0, 0,	//4
	51, 35, 0, 0,
	52, 36, 0, 0,
	53, 37, 0, 0,
	54, 94, 0, 0,	//8
	55, 38, 0, 0,
	56, 42, 0, 0,
	57, 40, 0, 0,
	48, 41, 0, 0,	//12
	45, 95, 0, 0,
	61, 43, 0, 0,
	8, 0, 0, 0,
	9, 0, 0, 0,		//16
	113, 81, 81, 0,
	119, 87, 87, 0,
	101, 69, 69, 0,
	114, 82, 82, 0,	//20
	116, 84, 84, 0,
	121, 89, 89, 0,
	117, 85, 85, 0,
	105, 73, 73, 0,	//24
	111, 79, 79, 0,
	112, 80, 80, 0,
	91, 123, 0, 0,
	93, 125, 0, 0,	//32
	10, 0, 0, 0,
	0, 0, 0, 0,	//lctrl -1
	97, 65, 65, 0,
	115, 83, 83, 0,	//36
	100, 68, 68, 0,
	102, 70, 70, 0,
	103, 71, 71, 0,
	104, 72, 72, 0,	//40
	106, 74, 74, 0,
	107, 75, 75, 0,
	108, 76, 76, 0,
	59, 58, 0, 0,	//44
	39, 34, 0, 0,
	96, 126, 0, 0,
	0, 0, 0, 0, 	//LSHIFT -1
	92, 124, 0, 0,	//48
	122, 90, 90, 0,
	120, 88, 88, 0,
	99, 67, 67, 0,
	118, 86, 86, 0,	//52
	98, 66, 66, 0,
	110, 78, 78, 0,
	109, 77, 77, 0,
	44, 60, 0, 0,
	46, 62, 0, 0,
	47, 63, 0, 0,
	0, 0, 0, 0, //RSHIFT -1
	42, 0, 0, 0,
	0, 0, 0, 0, //LALT -1
	32, 0, 0, 0,
	0, 0, 0, 0, //CAPS lock -1
	27, 1, 0, 0, //F1 -1 need to find out what this is
	27, 2, 0, 0, //F2 -1
	27, 3, 0, 0, //F3 -1
	27, 4, 0, 0, //F4 -1
	27, 5, 0, 0, //F5 -1
	27, 6, 0, 0, //F6 -1
	27, 7, 0, 0, //F7 -1
	27, 8, 0, 0, //F8 -1
	27, 9, 0, 0, //F9 -1
	27, 10, 0, 0, //F10 -1
	0, 0, 0, 0, //numlock -1
	0, 0, 0, 0, //scrolllock -1
	27, 11, 0, 20,	//needs numlock info - numpad 7
	27, 12, 0, 21, //needs numlock info - numpad 8
	27, 13, 0, 22, //needs numlock info - numpad 9
	45, 30, 0, 0, //- numpad
	27, 14, 0, 23, //needs numlock info - numpad 4
	53, 32, 0, 0, //numpad 5
	27, 15, 0, 24, //needs numlock info - numpad 6
	43, 34, 0, 0, //numpad +
	27, 16, 0, 25, //needs numlock info - numpad 1
	27, 17, 0, 26, //needs numlock info - numpad 2
	27, 18, 0, 27, //needs numlock info - numpad 3
	27, 19, 0, 28, //needs numlock info - numpad 0
	127, 0, 0, 46,	//numpad . delete
	0, 0, 0, 0, //RCTRL -1
	47, 0, 0, 0, //numpad /
	27, 29, 0, 0, //print screen -1
	27, 30, 0, 0, //f11 -1
	27, 31, 0, 0, //f12 -1
	0, 0, 0, 0, //RALT -1
	13, 0, 0, 0, //numpad enter
	27, 32, 0, 0, //home -1
	27, 33, 0, 0, //up -1
	27, 34, 0, 0,	//pgup -1
	27, 35, 0, 0,	//left -1
	27, 36, 0, 0,	//right -1
	27, 37, 0, 0,	//end -1
	27, 38, 0, 0,	//down -1
	27, 39, 0, 0,	//pgdown -1
	27, 40, 0, 0,	//insert -1
	127, 0, 0, 0, //delete -1
	0, 0, 0, 0,	//lwin -1
	0, 0, 0, 0,	//rwin -1
	0, 0, 0, 0,	//menu -1
	0, 0, 0, 0,	//pause/break -1	//105
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0,
	0, 0, 0, 0
};

//[0x1B:0x4F:0x53]

EXTERNC const char *ASCII_MULTIBYTE [] = {
"\x1B\x00",	//escape character (not actually a multi-byte, but this is here because of the way the code might act)
"\x00", //f1
"\x1B\x4F\x51\x00", //f2
"\x1B\x4F\x52\x00", //f3
"\x1B\x4F\x53\x00", //f4
"\x1B\x5B\x31\x36\x7E\x00", //f5
"\x1B\x5B\x31\x37\x7E\x00", //f6
"\x1B\x5B\x31\x38\x7E\x00", //f7
"\x1B\x5B\x31\x39\x7E\x00", //f8
"\x1B\x5B\x32\x30\x7E\x00", //f9
"\x00", //f10
"\x00", //numpad 7
"\x00", //numpad 8
"\x00", //numpad 9
"\x00", //numpad 4
"\x00", //numpad 6
"\x00", //numpad 1
"\x00", //numpad 2
"\x00", //numpad 3
"\x00", //numpad 0
"\x37\x00", //numpad 7 (with numlock on)
"\x38\x00", //numpad 8 (with numlock on)
"\x39\x00", //numpad 9 (with numlock on)
"\x34\x00", //numpad 4 (with numlock on)
"\x36\x00", //numpad 6 (with numlock on)
"\x31\x00", //numpad 1 (with numlock on)
"\x32\x00", //numpad 2 (with numlock on)
"\x33\x00", //numpad 3 (with numlock on)
"\x30\x00", //numpad 0 (with numlock on)
"\x00",	//print screen
"\x00",	//f11
"\x1B\x5B\x32\x34\x7E\x00",	//f12
"\x1B\x5B\x31\x7E\x00", //home
"\x1B\x5B\x41\x00",	//up
"\x1B\x5B\x35\x7E\x00", //pgup
"\x1B\x5B\x44\x00",	//left
"\x1B\x5B\x43\x00",	//right
"\x1B\x4F\x46\x00",	//end
"\x1B\x5B\x42\x00",	//down
"\x1B\x5B\x36\x7E\x00",	//pgdown
"\x1B\x5B\x32\x7E\x00",	//insert	element number 31
};

int wait_to_write()
{	//waits until the output buffer for the keyboard is clear
	//use before you send a command byte to port 0x60
	unsigned int counter = 0;
	while ((inportb(0x64) & 0x02) != 0)
	{
		Delay(10);
		counter += 10;
		if (counter == 100)
			return -1;
	}
	//waits until input buffer and output buffer are both empty
	return 0;
}

int wait_2_write()
{	//used when writing commands to port 0x64
	unsigned int counter = 0;
	while ((inportb(0x61) & 0x4) == 0x4)
	{	
		Delay(10);
		counter += 10;
		if (counter == 100)
			return -1;
	};	//maybe should be 0
	return 0;
}

int wait_to_read()
{	//waits until the output buffer has data in it
	//source says bit 5 may have to be checked alsos
	unsigned int counter = 0;
	while ((inportb(0x64) & 0x01) != 0x01)
	{
		Delay(10);
		counter += 10;
		if (counter == 100)
			return -1;
	}
	return 0;
}

//port 0x64
//Bitfields for keyboard controller read status (ISA, EISA):
//Bit(s)	Description	(Table P0398)
// 7	parity error on transmission from keyboard
// 6	receive timeout
// 5	transmit timeout
// 4	keyboard interface inhibited by keyboard lock
//	or by password server mode (IBM PS/2-286 [model bytes FCh/09h],
//	  "Tortuga" [model F8h/19h]) (see #00515 at INT 15/AH=C0h)
// 3	=1 data written to input register is command (PORT 0064h)
//	=0 data written to input register is data (PORT 0060h)
// 2	system flag status: 0=power up or reset	 1=selftest OK
// 1	input buffer full (input 60/64 has data for 8042)
//	no write access allowed until bit clears
// 0	output buffer full (output 60 has data for system)
//	bit is cleared after read access
//SeeAlso: PORT 0064h-R,#P0399,#P0400,#P0401


int read_data_port()
{	//reads a byte from the data port
	while (!(inportb(0x64) & 0x1)) { }
	return inportb(0x60);
}

void write_data_port(unsigned char value)
{
	while (inportb(0x64) & 0x2) { }
	outportb(value, 0x60);
}

//keyboard controller command byte
//read - send command 0x20, read the byte
//write - send command 0x60, then the new value
//bit 7 - unused set to 0
//bit 6 -translate
//bit 5 - mouse enable
//bit 4 - keyboard enable
//bit 3 - ignore keyboard lock
//bit 2 - system flag, cold boot = 0, warm boot = 1
//bit 1 - mouse interrupt enable (irq 12 is called when the mouse gets information)
//bit 0 - keyboard interrupt enable (irq 1 is called when enabled)

unsigned long num_elements_used;
unsigned long code_buffer[6] = {1,2,3,4,5,6};
unsigned long LastResponse;
unsigned long NumKeyInts;
extern unsigned long *keyb_handle;
struct message add_me;	//this will be used to add data to the system message buffer

void verify_scancode_receipt()
{	//resets via port 0x61, acknowledging receipt of the scancode
	unsigned int temp;
	temp = inportb(0x61);
	outportb(temp | 0x80, 0x61);
	Delay(10);	//add a delay in here to be safe
	outportb(temp, 0x61);
}

int init_keyboard()
{	//performs initialization of the keyboard
	num_elements_used = 0;
	unsigned int response, num_ints;
	display("\tInstalling interrupt handler\n");
	set_int_handler(&keyb_handle, 0x21);
	display("\tSetting keyboard to scancode set 2\n");
	//set keyboard to scancode set 2
	response = LastResponse;
	num_ints = NumKeyInts;
	write_data_port(0xF0);
	write_data_port(0x02);
	do
	{

		response = getResponse();
	} while (response == 0);
	if (response == 0xFE)
		display("\tFailed to set keyboard mode\n");
	add_me.who = KEYBOARD;
	add_me.fields = 2;
//	display("\tEnabling scancode translation\n");
	//enable translation, not working on some computers
//	if (wait_2_write() == -1)
//		return -1;
//	outportb(0x20, 0x64);
//	if (wait_to_write() == -1)
//		return -1;
//	do
//	{
//		response = getResponse();
//	} while (response == 0);
	if (wait_2_write() == -1)
		return -1;
	outportb(0x60, 0x64);
	write_data_port(0x43);
	display("Exiting keyboard initializer\n");
	return 0;
}

//format of the processed scancodes (4 bytes for each key event)
//2 for each (left and right key)
//ctrl, shift, alt, (more can be customized)
//make or break (press or release)
//caps lock
//num lock
//scroll lock
//

void postMakeCode(unsigned int code)
{	//short work for posting a code
	//flags such as shift will not be changed in this function
	//also the scancode buffer is cleared
	//this is where translation for VT100 stuff will be done
	//games will probably want to use the make and break codes, as well as anything that wants "extra" keyboard keys
		//like ctrl, shift, alt, etc
	//TODO: implement bounds check for the variable named code
	add_me.data1 = (add_me.data1 | code | MAKE);	//set the code and the make flag
	if (ASCII_TRANSLATE[(code & 0xFF) * 4] == 0)
	{	//key has no mapping
		add_me.data2 = 0;
	}
	else
	{	//this key has a mapping
		if (ASCII_TRANSLATE[(code & 0xFF) * 4] != 0x1B)
		{	//single byte
			if (((add_me.data1 & CAPSL) > 0) && (ASCII_TRANSLATE[(code & 0xFF) * 4 + 2] != 0))
			{	//caps lock key is engaged and the shift actually changes the keycode for this scancode
				if ((add_me.data1 & (LSHFT | RSHFT)) == 0)
				{	//a shift key is not being pressed
					//and caps lock is down
					add_me.data2 = ASCII_TRANSLATE[(code & 0xFF) * 4 + 2];
				}
				else
				{	//shift and caps lock = normal key output
					add_me.data2 = ASCII_TRANSLATE[(code & 0xFF) * 4];
				}
			}
			else if ((add_me.data1 & (LSHFT | RSHFT)) > 0)
			{	//a shift key is being pressed (and caps lock has no effect
				add_me.data2 = ASCII_TRANSLATE[(code & 0xFF) * 4 + 1];
			}
			else
			{	//neither the shift key or the caps lock is down
				add_me.data2 = ASCII_TRANSLATE[(code & 0xFF) * 4];
			}
		}
		else
		{
			if ((code & 0xFF) == 1)
			{	//the escape key is a single byte code
				add_me.data2 = 0x1B;
			}
			else
			{	//code is actually a multi-byte sequence (at least theoretically)
				//check to see if numlock is active and if the code cares about numlock
				add_me.data1 = add_me.data1 | MULTI;
				if (((add_me.data1 & NUMBL) > 0) && (ASCII_TRANSLATE[(code & 0xFF) * 4 + 3] != 0))
				{	//numlock active and the code has a different code when numlock is active
					add_me.data2 = (unsigned long)ASCII_MULTIBYTE[ASCII_TRANSLATE[(code & 0xFF) * 4 + 3]];
				}
				else
				{	//either numlock is not active or it doesn't matter if it is active
					add_me.data2 = (unsigned long)ASCII_MULTIBYTE[ASCII_TRANSLATE[(code & 0xFF) * 4 + 1]];
				}
			}
		}
	}
	add_system_event(&add_me);
	add_me.data1 = (add_me.data1 & 0xFFFFFF00);	//clear the key specific data
	add_me.data1 = (add_me.data1 & ~MULTI);
	add_me.data2 = 0;
	num_elements_used = 0;
}

void postBreakCode(unsigned int code)
{
	add_me.data1 = (add_me.data1 | code);
	add_me.data1 = (add_me.data1 & ~MAKE);	//set the code and clear the MAKE flag
	add_system_event(&add_me);
	add_me.data1 = (add_me.data1 & 0xFFFFFF00);	//clear the key specific data
	add_me.data2 = 0;
	num_elements_used = 0;
}

//0xE0 0x2A 0xE0, 0x53


//this function is called from assembly
void handleScancode(unsigned int code)
{	//only the bottom byte of code is non-zero
	//need to handle set 2 scancodes
	//PrintNumber(code);
	switch (num_elements_used)
	{ 
		case 0:
		{	//this is the first byte of the scancode
			switch ((code & 0xFF))
			{
				case 0xE0: case 0xE1:
					//scancode has more than one byte in it
					//save the byte and update the buffer
					num_elements_used = 1;
					code_buffer[0] = (code & 0xFF);
					break;
				case 0xEE: case 0x00: case 0xF0: case 0xFA: case 0xFC: case 0xFD:
				case 0xFE: case 0xFF:	//status bytes, don't bother posting a message
					num_elements_used = 0;
					break;
				case 0x2A:	//left shift key make
					add_me.data1 = (add_me.data1 | LSHFT);
					postMakeCode((code & 0xFF));
					break;
				case 0xAA:	//left shift key release
					add_me.data1 = (add_me.data1 & ~LSHFT);	//clear the LSHFT flag
					postBreakCode((code & 0xFF) - 0x80);
					break;
				case 0x36:	//right shift key make
					add_me.data1 = (add_me.data1 | RSHFT);
					postMakeCode((code & 0xFF));
					break;
				case 0xB6:	//right shift key release
					add_me.data1 = (add_me.data1 & ~RSHFT);	//clear the RSHFT flag
					postBreakCode((code & 0xFF) - 0x80);
					break;
				case 0x38:	//left alt key press
					add_me.data1 = (add_me.data1 | LALTT);
					postMakeCode((code & 0xFF));
					break;
				case 0x3A:	//caps lock key press
					if ((add_me.data1 & CAPSL) > 0)
						add_me.data1 = (add_me.data1 & ~CAPSL);
					else
						add_me.data1 = (add_me.data1 | CAPSL);
					postMakeCode((code & 0xFF));
					break;
				case 0xA8:	//left alt key release
					add_me.data1 = (add_me.data1 & ~LALTT);	//clear the LALTT flag
					postBreakCode((code & 0xFF) - 0x80);
					break;
				default:
					//this is for single byte scancodes
					//post message
					if ((code & 0x7F) > 0x58)
					{	//these need to be remapped (for now display an unknown key message)
						display("Unknown key:");
						PrintNumber((code & 0xFF));
						display("!");
					}
					else
					{	//these keys have a direct (more or less) map to the final set
						if ((code & 0xFF) < 0x80)
						{	//make code, the key was pressed
							postMakeCode((code & 0xFF));
						}
						else
						{	//break code
							postBreakCode((code & 0xFF) - 0x80);
						}
					}
					break;	
			}
			break;
		}
		case 1:
		{	//second byte of the scancode
			switch ((code & 0xFF))			
			{	//more than two bytes for a scancode
				case 0x2A: case 0xB7:
					num_elements_used = 2;
					code_buffer[1] = (code & 0xFF);
					break;
				//anything that passes this will be all of these will be 0xE0?? codes
				case 0x1C:	//numpad enter key (map to 0x5A)
					postMakeCode(0x5A);
					break;
				case 0x9C:	//numpad enter key release
					postBreakCode(0x5A);
					break;
				case 0x38:	//right alt key press
					add_me.data1 = (add_me.data1 | RALTT);	//set the code and the make flag
					postMakeCode(0x59);
					break;
				case 0xB8:	//right alt key release
					add_me.data1 = (add_me.data1 & ~RALTT);
					postBreakCode(0x59);
					break;
				case 0x1D:	//right ctrl key press
					if (code_buffer[0] == 0xE1)
					{	//this is the pause break key
						num_elements_used = 2;
						code_buffer[1] = (code & 0xFF);
					}
					else
					{
						add_me.data1 = (add_me.data1 | RCTRL);	//set the code and the make flag
						postMakeCode(0x54);
					}
					break;
				case 0x9D:	//right ctrl key release
					add_me.data1 = (add_me.data1 & ~RCTRL);
					postBreakCode(0x54);
					break;
				case 0x35:
					postMakeCode(0x55);
					break;
				case 0xB5:
					postBreakCode(0x55);
					break;
				case 0x37:
					postMakeCode(0x56);
					break;
				case 0x47: case 0x48: case 0x49:	//0x14
					postMakeCode((code & 0xFF) + 0x14);
					break;
				case 0xC7: case 0xC8: case 0xC9:
					postBreakCode((code & 0xFF) + 0x14 - 0x80);
					break;
				case 0x4B:
					postMakeCode(0x5E);
					break;
				case 0xCB:
					postBreakCode(0x5E);
					break;
				case 0x4D:
					postMakeCode(0x5F);
					break;
				case 0xCD:
					postBreakCode(0x5F);
					break;
				case 0x4F: case 0x50: case 0x51: case 0x52: case 0x53:
					postMakeCode((code & 0xFF) + 0x11);
					break;
				case 0xCF: case 0xD0: case 0xD1: case 0xD2: case 0xD3:
					postBreakCode((code & 0xFF) + 0x11 - 0x80);
					break;
				case 0x5B: case 0x5C: case 0x5D:
					postMakeCode((code & 0xFF) + 0xA);
					break;
				case 0xDB: case 0xDC: case 0xDD:
					postBreakCode((code & 0xFF) - 0x80 + 0xA);
					break;
				case 0xAA:	//nothing, it is a break code that is after the useful life of a key press
					num_elements_used = 0;
					break;
				default:
				{	//this is for 2-byte scancodes
						display("Unknown two byte scancode: ");
						PrintNumber(code_buffer[0]);
						display(",");
						PrintNumber((code & 0xFF));
						display("!");
					num_elements_used = 0;
					break;
				}
			}
			break;
		}
		case 2:
		{	//this is the third byte of the scancode
			switch ((code & 0xFF))
			{
				case 0xE0: case 0x45:
				{	//scancode has more than three bytes in it
					//save the byte and update the buffer
					num_elements_used = 3;
					code_buffer[2] = (code & 0xFF);
					break;
				}
				default:
				{	//this is for three byte scancodes
						display("Unknown three byte scancode: ");
						PrintNumber(code_buffer[0]);
						display(",");
						PrintNumber(code_buffer[1]);
						display(",");
						PrintNumber((code & 0xFF));
						display("!");
					num_elements_used = 0;
					break;	
				}
			}
			break;
		}
		case 3:
		{	//this is the fourth byte of the scancode
			switch ((code & 0xFF))
			{
				case 0xE1:
				{	//scancode has more than four bytes in it
					//save the byte and update the buffer
					num_elements_used = 4;
					code_buffer[3] = (code & 0xFF);
					break;
				}
				case 0x47: case 0x48: case 0x49:	//0x14
					postMakeCode((code & 0xFF) + 0x14);
					break;
				case 0xC7: case 0xC8: case 0xC9:
					postBreakCode((code & 0xFF) + 0x14 - 0x80);
					break;
				case 0x4B:
					postMakeCode(0x5E);
					break;
				case 0xCB:
					postBreakCode(0x5E);
					break;
				case 0x4D:
					postMakeCode(0x5F);
					break;
				case 0xCD:
					postBreakCode(0x5F);
					break;
				case 0x4F: case 0x50: case 0x51: case 0x52: case 0x53:
					postMakeCode((code & 0xFF) + 0x11);
					break;
				case 0xCF: case 0xD0: case 0xD1: case 0xD2: case 0xD3:
					postBreakCode((code & 0xFF) + 0x11 - 0x80);
					break;
				case 0x37:
					postMakeCode(0x56);
					break;
				case 0xAA:
					postBreakCode(0x56);
					break;
				default:
				{	//this is for four byte scancodes
						display("Unknown four byte scancode: ");
						PrintNumber(code_buffer[0]);
						display(",");
						PrintNumber(code_buffer[1]);
						display(",");
						PrintNumber(code_buffer[2]);
						display(",");
						PrintNumber((code & 0xFF));
						display("!");
					num_elements_used = 0;
					break;	
				}
			}
			break;
		}
		case 4:
		{	//this is the fifth byte of the scancode
			switch ((code & 0xFF))
			{
				case 0x9D:
				{	//scancode has more than four bytes in it
					//save the byte and update the buffer
					num_elements_used = 5;
					code_buffer[4] = (code & 0xFF);
					break;
				}
				default:
				{	//this is for five byte scancodes
						display("Unknown five byte scancode: ");
						PrintNumber(code_buffer[0]);
						display(",");
						PrintNumber(code_buffer[1]);
						display(",");
						PrintNumber(code_buffer[2]);
						display(",");
						PrintNumber(code_buffer[3]);
						display(",");
						PrintNumber((code & 0xFF));
						display("!");
					num_elements_used = 0;
					break;	
				}
			}
			break;
		}
		case 5:
		{	//this is the sixth byte of the scancode
			switch ((code & 0xFF))
			{
				case 0xC5:
					postMakeCode(0x68);
					break;
				default:
				{	//this is for six byte scancodes
						display("Unknown six byte scancode: ");
						PrintNumber(code_buffer[0]);
						display(",");
						PrintNumber(code_buffer[1]);
						display(",");
						PrintNumber(code_buffer[2]);
						display(",");
						PrintNumber(code_buffer[3]);
						display(",");
						PrintNumber(code_buffer[4]);
						display(",");
						PrintNumber((code & 0xFF));
						display("!");
					num_elements_used = 0;
					break;	
				}
			}
			break;
		}
		default:
		{	//this means an error happened
			display("Error in keyboard handler. Value = ");
			PrintNumber(num_elements_used);
			display("\n");
			break;
		}
	}
}


	

/*keycode order (translated)
//everything that doesnt have an ASCII code right now could potentially have an escape sequence for it
(translated value) [ASCII value] [esc:esc:esc:esc]
esc (0) [27]
1! [49, 33]
2@ [50, 64]
3# [51, 35]
4$ [52, 36]
5% (5) [53, 37]
6^ [54, 94]
7& [55, 38]
8* [56, 42]
9( [57, 40]
0)(10) [48, 41]
-_ [45, 95]
=+ [61, 43]
backspace [8]
tab [9]
qQ (15) [113, 81]
wW [119, 87]
eE [101, 69]
rR [114, 82]
tT [116, 84]
yY (20) [121, 89]
uU [117, 85]
iI [105, 73]
oO [111, 79]
pP [112, 80]
[{ (25) [91, 123]
]} [93, 125]
enter [13]
lCTRL
aA [97, 65]
sS (30) [115, 83]
dD [100, 68]
fF [102, 70]
gG [103, 71]
hH [104, 72]
jJ (35) [106, 74]
kK [107, 75]
lL [108, 76]
;: [59, 58]
'" [39, 34]
`~ (40) [96, 126]
LSHIFT
\| [92, 124]
zZ [122, 90]
xX [120, 88]
cC (45) [99, 67]
vV [118, 86]
bB [98, 66]
nN [110, 78]
mM [109, 77]
,< (50) [44, 60]
.> [46, 62]
/? [47, 63]
RSHIFT
* [42]
LALT (55)
' ' [32]
capslock
f1 
f2 [0x1B:0x4F:0x51]
f3 (60) [0x1B:0x4F:0x52]
f4 [0x1B:0x4F:0x53]
f5 [0x1B:0x5B:0x31:0x36:0x7E]
f6 [0x1B:0x5B:0x31:0x37:0x7E]
f7 [0x1B:0x5B:0x31:0x38:0x7E]
f8 (65) [0x1B:0x5B:0x31:0x39:0x7E]
f9.[0x1B:0x5B:0x32:0x30:0x7E]
f10
numlock
scrolllock	//items below this are switched by numlock lock, not capslock or shift
home 7 (70) [x, 55]	//these values need to be tested with a real keyboard
up 8 [x, 56]
pageup 9 [x, 60]
- [45]
left 4 [x, 52]
5 (75) [53]
right 6 [x, 54]
+ [43]
end 1 [x, 49]
down 2 [x, 50]
pagedown 3 (80) [x, 51]
insert 0 [x, 48]
del . [127, 46]
RCTRL (0x54) [x]	//items above this are controlled by num lock
/ (0x55) [47]
printscreen (0x56) (85)
f11 (0x57)
f12 (0x58) [0x1B:0x5B:0x32:0x34:0x7E]
RALT (0x59)
numpad_enter (0x5A) [13]
home (0x5B) (90) [0x1B:0x5B:0x31:0x7E]
up [0x1B:0x5B:0x41]
pgup [0x1B:0x5B:0x35:0x7E]
left (0x5E) [0x1B:0x5B:0x44]
right [0x1B:0x5B:0x43]
end (95) [0x1B:0x4F:0x46]
down (0x61) [0x1B:0x5B:0x42]
pgdwn [0x1B:0x5B:0x36:0x7E]
insert [0x1B:0x5B:0x32:0x7E]
del (0x64) [127]
lwin (100)
rwin
menu (0x67)
pause/break (0x68)
*/

//create a conversion table for the keyboard right here

enum
{
ASCII_NUL = 0,
ASCII_SOH,	//start of heading
ASCII_STX,	//start of text
ASCII_ETX,	//end of text
ASCII_EOT,	//end of transmission
ASCII_ENQ,	//enquuiry
ASCII_ACK,	//acknowledge
ASCII_BEL,	//bell
ASCII_BS,		//backspace
ASCII_TAB,
ASCII_LF,		//NL line feed, new line
ASCII_VT,		//vertical tab
ASCII_FF,		//NP form feed, new page
ASCII_CR,		//carriage return
ASCII_SO,		//shift out
ASCII_SI,		//shift in
ASCII_DLE,	//data link escape
ASCII_DC1,	//device control 1
ASCII_DC2,	//device control 2
ASCII_DC3,	//device control 3
ASCII_DC4,	//device control 4
ASCII_NAK,	//negative acknowledgement
ASCII_SYN,	//synchronous idle
ASCII_ETB,	//end of transmission block
ASCII_CAN,	//cancel
ASCII_EM,		//end of medium
ASCII_SUB,	//substitute
ASCII_ESC,	//escape
ASCII_FS,		//file seperator
ASCII_GS,		//group seperator
ASCII_RS,		//record seperator
ASCII_US,		//unit seperator
ASCII_SPACE,
ASCII_EXCL,	// !
ASCII_DQ,	//"
ASCII_POUND,
ASCII_DOLLAR,
ASCII_PERCENT,
ASCII_AMP,
ASCII_TICK,
ASCII_OPAR,
ASCII_CPAR,
ASCII_MUL,
ASCII_ADD,
ASCII_COMMA,
ASCII_SUBTRACT,
ASCII_DOT,
ASCII_FSLASH,
ASCII_0,
ASCII_1,
ASCII_2,
ASCII_3,
ASCII_4,
ASCII_5,
ASCII_6,
ASCII_7,
ASCII_8,
ASCII_9,
ASCII_COLON,
ASCII_SEMCLN,
ASCII_LESSER,
ASCII_EQUAL,
ASCII_GREATER,
ASCII_QSTN,
ASCII_AT,
ASCII_A,
ASCII_B,
ASCII_C,
ASCII_D,
ASCII_E,
ASCII_F,
ASCII_G,
ASCII_H,
ASCII_I,
ASCII_J,
ASCII_K,
ASCII_L,
ASCII_M,
ASCII_N,
ASCII_O,
ASCII_P,
ASCII_Q,
ASCII_R,
ASCII_S,
ASCII_T,
ASCII_U,
ASCII_V,
ASCII_W,
ASCII_X,
ASCII_Y,
ASCII_Z,
ASCII_OBRACK,
ASCII_BSLASH,
ASCII_CBRACK,
ASCII_POW,	//^
ASCII_UNDERSCORE,
ASCII_BACKTICK,	//`
ASCII_LOWA,
ASCII_LOWB,
ASCII_LOWC,
ASCII_LOWD,
ASCII_LOWE,
ASCII_LOWF,
ASCII_LOWG,
ASCII_LOWH,
ASCII_LOWI,
ASCII_LOWJ,
ASCII_LOWK,
ASCII_LOWL,
ASCII_LOWM,
ASCII_LOWN,
ASCII_LOWO,
ASCII_LOWP,
ASCII_LOWQ,
ASCII_LOWR,
ASCII_LOWS,
ASCII_LOWT,
ASCII_LOWU,
ASCII_LOWV,
ASCII_LOWW,
ASCII_LOWX,
ASCII_LOWY,
ASCII_LOWZ,
ASCII_OCBRACK,	//{
ASCII_BAR,	//|
ASCII_CCBRACK,	//}
ASCII_TILDE,	//~
ASCII_DELETE
};
