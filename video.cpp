#include "video.h"
#include "spinlock.h"
#include "serial.h"

video vid;

video::video()
{
}

void video::display(const char *cp)
{
	enter_spinlock(SL_DISPLAY);
	const char *str = cp, *ch;
	for (ch = str; *ch; ch++)
	{
		put(*ch);
	}
	leave_spinlock(SL_DISPLAY);
}

void video::PrintNumber(unsigned int bob)
{	//this prints a 32 bit number (8 hex digits)
	enter_spinlock(SL_DISPLAY);
	unsigned int Temp = 0;
	unsigned int last_digit = 0;
	put('0');
	put('x');
	int counter = 7;
	for (counter = 7; counter >= 0; counter--)
	{	//this is a countdown, because we write the most signifigant nibble first
		Temp = ((bob >> (counter * 4)) & 0xF);
		if (last_digit == 0)
		{
			last_digit = Temp;
		}
		if (last_digit != 0)
		{
			if (Temp > 9)
			{
				Temp += ('A' - 10);
			}
			else
			{
				Temp += '0';
			}
			put((unsigned char)(Temp));
		}
	}
	if (last_digit == 0)
	{
		put('0');
		put('0');
	}
	leave_spinlock(SL_DISPLAY);
}

void video::put(unsigned char c)
{
	//enter_spinlock(SL_DIS2);
	unsigned short *videomem = (unsigned short*) 0xB8000;
	if (pos >= 80)
	{
		kellogs.write_serial(13);
		pos = 0;
		off += 80;
	}
	if (off >= (80 * 25))
	{
					//to scroll the screen, read all data except the top row from the screen
					//then write it back, with the bottom row being "clear"
		scroll_screen(); 		//should scroll the screen, but for now, just clear
		off = (80 * 24);
	}
	//time to check for special ASCII values like newline and tab
	switch(c) 
	{
		case 0: case 1: case 2: case 3: //do nothing becuase these are non printing characters that mean nothing
		case 4: case 5: case 6: case 31: //these will eventually cause a beep (beep)
		case 11: case 15: case 16: case 17: case 18: case 19: case 20: case 21:
		case 22: case 23: case 24: case 25: case 26: case 27: case 28: case 29:
		case 30:
		{
			break;
		}
		case 7:
		{	//beep
			kellogs.write_serial(7);
			break;
		}
		case 8:	//backspace (this will be weird)
		{	//if not on the beginning of a line, make the previous spot a space and make the current space the previous space
			kellogs.write_serial(8);
			if (pos != 0)
			{
				pos--;
				videomem[off + pos] = (unsigned char) ' ' | 0x0700;
			}
			else if (pos == 0)
			{	//decrease the current spot until we find a non blank spot, then go to the spot after that one
				pos = 79;
				off -= 80;
				while ((videomem[off + pos] == ' '))
				{
					pos--;
				}
				videomem[off + pos] = (unsigned char) ' ' | 0x0700;
			}
			break;
		}
		case 127:	//delete (this one will be weird)
		{
			kellogs.write_serial(127);
			break;
		}
		case 9:	//tab to four spaces (at least one space required)
		{
			kellogs.write_serial(9);
			if (pos > 75)	//pointless to tab to the last character, newline instead
			{	//we wont end up filling up the screen all the way yet
				pos = 0;
				off += 80;
				break;	//don't tab
			}
			do
			{
				videomem[off + pos] = (unsigned char) ' ' | 0x0700;	//one space as required
				pos++;
			} while ((pos % 4) != 0);
			break;	//this is very important
		}
		case 10:	//this is newline (or is this just bring cursor to beginning of the line)
		{		//easy to test
			kellogs.write_serial(13);
			pos = 0;
			off += 80;
			break;
		}
		case 12:	//maybe we should clear the screen?
		{
			break;
		}
		case 13:	//carriage return 		
		{
			kellogs.write_serial(13);
			pos = 0;
			break;
		}
		default:	//all printable characters
		{	
			kellogs.write_serial(c);
			videomem[off + pos] = (unsigned char) c | 0x0700;
			pos++;
			break;
		}
	}
	//leave_spinlock(SL_DIS2);
}

EXTERNC void video::clear_screen()
{	//this also is an initializer
	unsigned short *videomem = (unsigned short*) 0xB8000;
	int counter;
	for (counter = 0; counter < (80 * 25); counter++)
	{
		videomem[counter] = (unsigned char) ' ' | 0x0700;
	}
	off = 0; pos = 0;
}

void video::scroll_screen()
{
	unsigned short *destination = (unsigned short*) 0xB8000;
	unsigned short *source = (unsigned short*) 0xB8000 + 80;
	unsigned int counter;
	for (counter = 0; counter < (79 * 25); counter++)
	{
		destination[counter] = source[counter];
	}
	for(;counter < (80 * 25); counter++)
	{
		destination[counter] = (unsigned char) ' ' | 0x0700;
	}
}

void display(const char *cp)
{
	vid.display(cp);
	//make call to the class function
}

void PrintNumber(unsigned int bob)
{
	vid.PrintNumber(bob);
	//transfer the call appropriately
}

void put(unsigned char letter)
{
	vid.put(letter);
}
//prints a single character to the screen
void clear_screen()
{
	vid.clear_screen();
}
