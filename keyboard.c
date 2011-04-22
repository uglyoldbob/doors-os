#include "keyboard.h"
#include "message.h"
#include "entrance.h"

//unsigned int BootType;	//warm or cold boot; 0 = cold, 1 = warm
	//not valid on all computers

//the keyboard is initalized to use translated scancode set 2 (sources I have read say it is practically gauranteed to work on all keyboards (sets 1 and 3 might not work on all for some weird reason)
//the keyboard only sends codes one byte at a time, so each scancode is processed with a buffer
//when the last byte of a scancode is recieved, it is translated and placed in the system notification buffer
	//via a spinlock aware function


void wait_to_write()
{	//waits until the output buffer for the keyboard is clear
	//use before you send a command byte to port 0x60
	while ((inportb(0x64) & 0x03) != 0);
	//waits until input buffer and output buffer are both empty
}

void wait_2_write()
{	//used when writing commands to port 0x64
	while ((inportb(0x61) & 0x4) == 0x4);	//maybe should be 0
}

void wait_to_read()
{	//waits until the output buffer has data in it
	//source says bit 5 may have to be checked alsos
	while ((inportb(0x64) & 0x01) != 0x01);
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

extern unsigned int code_buffer[6];	//used to store the scancodes from the keyboard
extern unsigned int num_elements_used;
struct message add_me;	//this will be used to add data to the system message buffer

void verify_scancode_receipt()
{	//resets via port 0x61, acknowledging receipt of the scancode
	unsigned int temp;
	temp = inportb(0x61);
	outportb(temp | 0x80, 0x61);
	Delay(10);	//add a delay in here to be safe
	outportb(temp, 0x61);
}

void init_keyboard()
{	//performs initialization of the keyboard
	num_elements_used = 0;
	unsigned int response;
	display("\tSetting keyboard to scancode set 2\n");
	//set keyboard to scancode set 2
//	wait_to_write();
//	outportb(0xF0, 0x60);
//	wait_to_write();
//	outportb(0x2, 0x60);
//	do
//	{
//		response = getResponse();
//	} while (response == 0);
//	if (response == 0xFE)
//		display("\tFailed to set keyboard mode\n");
	add_me.who = KEYBOARD;
	add_me.fields = 1;
	display("\tEnabling scancode translation\n");
	//enable translation, not working on some computers
	wait_2_write();
	outportb(0x20, 0x64);
	wait_to_write();
	do
	{
		response = getResponse();
	} while (response == 0);

	response = 0x43;	//enable mouse, keyboard, scancode conversion
	wait_2_write();
	outportb(0x60, 0x64);
	wait_to_write();
	outportb(response, 0x60);
	wait_to_write();
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
	add_me.data1 = (add_me.data1 | code | MAKE);	//set the code and the make flag
	add_system_event(&add_me);
	add_me.data1 = (add_me.data1 & 0xFFFFFF00);	//clear the key specific data
	num_elements_used = 0;
}

void postBreakCode(unsigned int code)
{
	add_me.data1 = (add_me.data1 | code);
	add_me.data1 = (add_me.data1 & ~MAKE);	//set the code and clear the MAKE flag
	add_system_event(&add_me);
	add_me.data1 = (add_me.data1 & 0xFFFFFF00);	//clear the key specific data
	num_elements_used = 0;
}

void handleScancode(unsigned int code)
{	//only the bottom byte of code is non-zero
	//need to handle set 2 scancodes
	//PrintNumber(code);
	switch (num_elements_used)
	{
		case 0:
		{	//this is the first byte of the scancode
			switch (code)
			{
				case 0xE0: case 0xE1:
					//scancode has more than one byte in it
					//save the byte and update the buffer
					num_elements_used = 1;
					code_buffer[0] = code;
					break;
				case 0xEE: case 0x00: case 0xF0: case 0xFA: case 0xFC: case 0xFD:
				case 0xFE: case 0xFF:	//status bytes, don't bother posting a message
					num_elements_used = 0;
					break;
				case 0x2A:	//left shift key make
					add_me.data1 = (add_me.data1 | LSHFT);
					postMakeCode(code);
					break;
				case 0xAA:	//left shift key release
					add_me.data1 = (add_me.data1 & ~LSHFT);	//clear the LSHFT flag
					postBreakCode(code - 0x80);
					break;
				case 0x36:	//right shift key make
					add_me.data1 = (add_me.data1 | RSHFT);
					postMakeCode(code);
					break;
				case 0xB6:	//right shift key release
					add_me.data1 = (add_me.data1 & ~RSHFT);	//clear the RSHFT flag
					postBreakCode(code - 0x80);
					break;
				case 0x38:	//left alt key press
					add_me.data1 = (add_me.data1 | LALTT);
					postMakeCode(code);
					break;
				case 0xA8:	//left alt key release
					add_me.data1 = (add_me.data1 & ~LALTT);	//clear the LALTT flag
					postBreakCode(code - 0x80);
					break;
				default:
					//this is for single byte scancodes
					//post message
					if ((code & 0x7F) > 0x58)
					{	//these need to be remapped (for now display an unknown key message)
						display("Unknown key:");
						PrintNumber(code);
						display("!");
					}
					else
					{	//these keys have a direct (more or less) map to the final set
						if (code < 0x80)
						{	//make code, the key was pressed
							postMakeCode(code);
						}
						else
						{	//break code
							postBreakCode(code - 0x80);
						}
					}
					break;	
			}
			break;
		}
		case 1:
		{	//second byte of the scancode
			switch (code)			
			{	//more than two bytes for a scancode
				case 0x2A: case 0xB7:
					num_elements_used = 2;
					code_buffer[1] = code;
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
						code_buffer[1] = code;
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
					postMakeCode(code + 0x14);
					break;
				case 0xC7: case 0xC8: case 0xC9:
					postBreakCode(code + 0x14 - 0x80);
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
					postMakeCode(code + 0x11);
					break;
				case 0xCF: case 0xD0: case 0xD1: case 0xD2: case 0xD3:
					postBreakCode(code + 0x11 - 0x80);
					break;
				case 0x5B: case 0x5C: case 0x5D:
					postMakeCode(code + 0xA);
					break;
				case 0xDB: case 0xDC: case 0xDD:
					postBreakCode(code - 0x80 + 0xA);
					break;
				case 0xAA:	//nothing, it is a break code that is after the useful life of a key press
					num_elements_used = 0;
					break;
				default:
				{	//this is for 2-byte scancodes
						display("Unknown two byte scancode: ");
						PrintNumber(code_buffer[0]);
						display(",");
						PrintNumber(code);
						display("!");
					num_elements_used = 0;
					break;
				}
			}
			break;
		}
		case 2:
		{	//this is the third byte of the scancode
			switch (code)
			{
				case 0xE0: case 0x45:
				{	//scancode has more than three bytes in it
					//save the byte and update the buffer
					num_elements_used = 3;
					code_buffer[2] = code;
					break;
				}
				default:
				{	//this is for three byte scancodes
						display("Unknown three byte scancode: ");
						PrintNumber(code_buffer[0]);
						display(",");
						PrintNumber(code_buffer[1]);
						display(",");
						PrintNumber(code);
						display("!");
					num_elements_used = 0;
					break;	
				}
			}
			break;
		}
		case 3:
		{	//this is the fourth byte of the scancode
			switch (code)
			{
				case 0xE1:
				{	//scancode has more than four bytes in it
					//save the byte and update the buffer
					num_elements_used = 4;
					code_buffer[3] = code;
					break;
				}
				case 0x47: case 0x48: case 0x49:	//0x14
					postMakeCode(code + 0x14);
					break;
				case 0xC7: case 0xC8: case 0xC9:
					postBreakCode(code + 0x14 - 0x80);
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
					postMakeCode(code + 0x11);
					break;
				case 0xCF: case 0xD0: case 0xD1: case 0xD2: case 0xD3:
					postBreakCode(code + 0x11 - 0x80);
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
						PrintNumber(code);
						display("!");
					num_elements_used = 0;
					break;	
				}
			}
			break;
		}
		case 4:
		{	//this is the fifth byte of the scancode
			switch (code)
			{
				case 0x9D:
				{	//scancode has more than four bytes in it
					//save the byte and update the buffer
					num_elements_used = 5;
					code_buffer[4] = code;
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
						PrintNumber(code);
						display("!");
					num_elements_used = 0;
					break;	
				}
			}
			break;
		}
		case 5:
		{	//this is the sixth byte of the scancode
			switch (code)
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
						PrintNumber(code);
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
esc (0)
1!
2@
3#
4$
5%
6^
7&
8*
9(
0)
-_
=+
backspace
tab
qQ
wW
eE
rR
tT
yY
uU
iI
oO
pP
[{
]}
enter
lCTRL
aA
sS
dD
fF
gG
hH
jJ
kK
lL
;:
'"
`~
LSHIFT
\|
zZ
xX
cC
vV
bB
nN
mM
,<
.>
/?
RSHIFT
*
LALT
' '
capslock
f1
f2
f3
f4
f5
f6
f7
f8
f9
f10
numlock
scrolllock
home 7
up 8
pageup 9
-
left 4
5
right 6
+
end 1
down 2
pagedown 3
insert 0
del .
RCTRL (0x54)
/ (0x55)
printscreen (0x56)
f11 (0x57)
f12 (0x58)
RALT (0x59)
numpad_enter (0x5A)
home (0x5B)
up
pgup
left (0x5E)
right
end
down (0x61)
pgdwn
insert
del (0x64)
lwin
rwin
menu (0x67)
pause/break (0x68)
*/
