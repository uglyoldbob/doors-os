#include "tss.h"
#include "memory.h"

int add_task_next(const struct TSS *new_task, struct task *list)
{	//add task new to list (it becomes the next task)
	//don't change the current spot in the list
	struct task *intermediate;
	intermediate = malloc (sizeof(struct task));
	intermediate->me = malloc ( sizeof(struct TSS) );	
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
	PrintNumber(getEIP());
	display(" debug here\n");
	intermediate = malloc (sizeof(struct task));	
	intermediate->me = malloc ( sizeof(struct TSS) );
//	display("add_task: malloc success\n");
	intermediate->next = list;
	intermediate->previous = list->previous;
//	display("add_task: setting up new object success\n");
	list->previous->next = intermediate;
	list->previous = intermediate;
	if (list->next == list)
		list->next = intermediate;
//	display("add_task: Setting pointers to new object success\n");
	display("Destination: ");
	PrintNumber(intermediate->me);
	display("\n");
	memcopy(intermediate->me, new_task, sizeof(struct TSS));
	display("Source: ");
	PrintNumber(new_task);
	display("\n");
	PrintNumber(new_task->cs);
	display(": cs\n");
//	PrintNumber(list->previous);
//	display(", ");
//	PrintNumber(list);
//	display(", ");
//	PrintNumber(list->next);
//	display("\nadd_task: full success\n");

	PrintNumber(list->previous);
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
	display("\n");

	return 0;
}

int remove_current_task(struct task *list)
{	//removes the current task and advances to the next task
	struct task *intermediate;
	list->previous->next = list->next;
	list->next->previous = list->previous;
	intermediate = list;
	list = list->next;
	free (intermediate->me);
	free (intermediate);
	return 0;
}

int remove_next_task(struct task *list)
{	//removes the next task, the current task remains unchanged
	struct task *intermediate;
	intermediate = list->next;
	list->next = list->next->next;
	list->next->previous = list;
	free (intermediate->me);
	free (intermediate);
	return 0;
}

int remove_previous_task(struct task *list)
{	//remove the previous task, again, the current task remains unchanged
	struct task *intermediate;
	intermediate = list->previous;
	list->previous = list->previous->previous;
	list->previous->next = list;
	free (intermediate->me);
	free (intermediate);
	return 0;
}

int init_first_task(struct task *list)
{	//initializes the first task (main)
	list->next = list;
	list->previous = list;
	list->me = malloc (sizeof(struct TSS));
	list->me->cs = 0x08;
	list->me->ds = 0x10;
	list->me->es = 0x10;
	list->me->fs = 0x10;
	list->me->gs = 0x10;
	list->me->ss = 0x10;
	list->me->cr3 = getCR3();
	list->me->ldt_segment_selector = 0;
	PrintNumber(get_current_tss());
	display(" is where information for the first task is being copied to\n");
	memcopy(get_current_tss(), list->me, sizeof(struct TSS));
	//the TSS data for the current task is meaningless
	//TSS data is only useful when a task has been suspended/(is not currently executing)
}

int next_state(unsigned long valid_previous, struct task *list, unsigned long *multi_tss)
{	//store data for the previous task
	//load data for the next task
//	if (list->previous == list->next)
//		return 0;

//	display("\nBegin next_state: ");
//	PrintNumber(list);
//	display(" @ ");
//	PrintNumber(multi_tss);
//	display("\n");
	if (valid_previous == 1)
		memcopy(list->previous->me, multi_tss, sizeof(struct TSS));
	list = list->next;
	memcopy(multi_tss, list->me, sizeof(struct TSS));
}

unsigned long get_sys_tasks()
{	return sys_tasks;	}

void secondary_task()
{
	int counter;
	while (1)
	{
		counter++;
		if (counter == 0x1000)
		{
			display("Task 2.");
			counter = 0;
		}
		Delay(1);
	}
}
