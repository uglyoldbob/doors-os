#include "video.h"
Video::Video()
{
	pos = 0;
	off = 0;
	videomem = (unsigned short*) 0xB8000;
	while ((videomem[pos + off] & 0xFF) != 0x20)
	{
		off += 80;
	}
}

Video::~Video() {}

void Video::clear()	//does not work
{
	off = 160;		
	pos = 0;
	for (int counter = 0; counter < (80 * 25); counter++)
	{
		put(' ');
	}
	off = 0;
	pos = 0;
}

void Video::write(char *cp)	//must be null terminated
{
	char *str = cp, *ch;
	for (ch = str; *ch; ch++)
	{
		put(*ch);
	}
}

void Video::put(char c)
{
	if (pos >= 80)
	{
		pos = 0;
		off += 80;
	}
	if (off >= (80 * 25))
	{
					//to scroll the screen, read all data except the top row from the screen
					//then write it back, with the bottom row being "clear"
		clear(); 		//should scroll the screen, but for now, just clear
	}
	//time to check for special ASCII values like newline and tab
	switch(c) {
		case 0: case 1: case 2: case 3: //do nothing becuase these are non printing characters that mean nothing
		case 4: case 5: case 6: case 7: //these will eventually cause a beep (beep)
		case 11: case 15: case 16: case 17: case 18: case 19: case 20: case 21:
		case 22: case 23: case 24: case 25: case 26: case 27: case 28: case 29:
		case 30: case 31:
		{
			break;
		}
		case 8:	//backspace (this will be weird)
		{	
			break;
		}
		case 127:	//delete (this one will be weird)
		{
			break;
		}
		case 9:	//tab to four spaces (at least one space required)
		{
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
			pos = 0;
			break;
		}
		default:	//all printable characters
		{
			videomem[off + pos] = (unsigned char) c | 0x0700;
			pos++;
			break;
		}
	}
}
