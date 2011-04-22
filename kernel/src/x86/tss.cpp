#include "tss.h"
#include "memory.h"
#include "video.h"
#include "entrance.h"

struct task *sys_tasks;		//stores the current place in the task list

int add_task_next(const struct TSS *new_task, struct task *list)
{	//add task new to list (it becomes the next task)
	//don't change the current spot in the list
	struct task *intermediate;
	intermediate = (struct task*)kmalloc (sizeof(struct task));
	intermediate->me = (struct TSS*)kmalloc ( sizeof(struct TSS) );	
	intermediate->previous = list;
	intermediate->next = list->next;
	list->next = intermediate;
	list->next->previous = intermediate;
	*intermediate->me = *new_task;
	return 0;
}

int add_task_before(const struct TSS *new_task, struct task *list)
{	//add task new to list (it becomes the previous task)
	//don't change the current spot in the list
//	display("add_task: start\n");

	struct task *intermediate;
	/*PrintNumber(getEIP());
	display(" debug here\n");*/
	intermediate = (struct task*)kmalloc (sizeof(struct task));	
	intermediate->me = (struct TSS*)kmalloc ( sizeof(struct TSS) );

	display("Task information located at: ");
	PrintNumber((unsigned int)intermediate);
	display("\nTask tss located at: ");
	PrintNumber((unsigned int)intermediate->me);
	display("\n");

//	display("add_task: kmalloc success\n");
	intermediate->next = list;
	intermediate->previous = list->previous;
//	display("add_task: setting up new object success\n");
	list->previous->next = intermediate;
	list->previous = intermediate;
	if (list->next == list)
		list->next = intermediate;
//	display("add_task: Setting pointers to new object success\n");
	/*display("Destination: ");
	PrintNumber(intermediate->me);
	display("\n");*/

	//intermediate->me = new_task;
	memcopy(intermediate->me, new_task, sizeof(struct TSS));

/*	display("Source: ");
	PrintNumber(new_task);
	display("\n");
	PrintNumber(new_task->cs);
	display(": cs\n");*/
	PrintNumber((unsigned int)list->previous->previous);
	display(", ");
	PrintNumber((unsigned int)list->previous);
	display(", ");
	PrintNumber((unsigned int)list);
	display(", ");
	PrintNumber((unsigned int)list->next);
	display(", ");
	PrintNumber((unsigned int)list->next->next);
	display("\n");
//	display("\nadd_task: full success\n");

/*	PrintNumber(list->previous);
	display(" ");
	PrintNumber(list);
	display(" ");
	PrintNumber(list->next);
	display("\n");
	PrintNumber(list->next->previous);
	display(" ");
	PrintNumber(list->next);
	display(" ");
	PrintNumber(list->next->next);
	display("\n");*/

	return 0;
}

struct task * remove_current_task(struct task *list)
{	//removes the current task and advances to the next task
	struct task *intermediate;
	list->previous->next = list->next;
	list->next->previous = list->previous;
	intermediate = list;
	list = list->next;
	kfree (intermediate->me);
	kfree (intermediate);
	return list;
}

int remove_next_task(struct task *list)
{	//removes the next task, the current task remains unchanged
	struct task *intermediate;
	intermediate = list->next;
	list->next = list->next->next;
	list->next->previous = list;
	kfree (intermediate->me);
	kfree (intermediate);
	return 0;
}

int remove_previous_task(struct task *list)
{	//remove the previous task, again, the current task remains unchanged
	struct task *intermediate;
	intermediate = list->previous;
	list->previous = list->previous->previous;
	list->previous->next = list;
	kfree (intermediate->me);
	kfree (intermediate);
	return 0;
}

int init_first_task(struct task *list)
{	//initializes the first task (main)
	list->next = list;
	list->previous = list;
	list->me = (struct TSS*)kmalloc (sizeof(struct TSS));

	display("Task information located at: ");
	PrintNumber((unsigned int)list);
	display("\nTask tss located at: ");
	PrintNumber((unsigned int)list->me);
	display("\n");

	list->me->cs = 0x08;
	list->me->ds = 0x10;
	list->me->es = 0x10;
	list->me->fs = 0x10;
	list->me->gs = 0x10;
	list->me->ss = 0x10;
	list->me->cr3 = getCR3();
	list->me->ldt_segment_selector = 0;
	list->me->io_map_base_address = 0;
	list->me->previous_task = 0;
	memcopy(get_current_tss(), list->me, sizeof(struct TSS));
	//the TSS data for the current task is meaningless
	//TSS data is only useful when a task has been suspended/(is not currently executing)
}

//this function is called from assembly
struct task * next_state(unsigned long valid_previous, struct task *list, struct TSS *multi_tss, struct TSS *prev_tss)
{	//store data for the previous task
	//load data for the next task
	/*display("\nTask: ");
	PrintNumber(list->me);
	display("\n");*/
	/*display(", Previous tss: ");
	PrintNumber(prev_tss);
	display(", Current tss: ");
	PrintNumber(multi_tss);
	display("\n");*/

	if (valid_previous == 1)
	{
		/*display("Backup state from ");
		PrintNumber(prev_tss);
		display(" to: ");
		PrintNumber(list->previous->me);
		display("\nPrevious CS: ");
		PrintNumber(prev_tss[19]);*/
		
		//the cpu is properly storing the previous task
		//memcopy is overwriting the information that we want to keep
		memcopy(list->previous->me, prev_tss, sizeof(struct TSS));
		
		/*display(" (");
		PrintNumber(list->previous->me->cs);
		display(")\n");
		display("The other TSS CS: ");
		PrintNumber(multi_tss[19]);
		display("\n");*/
	}
	else
	{
		//display("\n");
	}
	/*display("Copy task tss from ");
	PrintNumber(list->next->me);
	display(" to ");
	PrintNumber(prev_tss);
	display("\nNew cs:");
	PrintNumber(list->next->me->cs);
	display(" (");*/
	memcopy(prev_tss, list->next->me, sizeof(struct TSS));
		//the memory management routines are assigning the addresses for the tss structures to other people
			//this is bad
	/*PrintNumber(prev_tss[19]);
	display(")\n");*/
	return list->next;
}

void secondary_task()
{
	unsigned long counter;
	while (1)
	{
		counter++;
		if (counter == 0x10000)
		{
//			display("Task 2.");
			Delay(10000);
			counter = 0;
		}
	}
}
