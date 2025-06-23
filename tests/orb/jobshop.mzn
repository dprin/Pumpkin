%-----------------------------------------------------------------------------%
% jobshop.mzn (modified to use disjunctive constraint)
%-----------------------------------------------------------------------------%

%-----------------------------------------------------------------------------%
% Model parameters.
%

int: n_machines;                        % The number of machines.
int: n_jobs;                            % The number of jobs.
int: n_tasks_per_job = n_machines;     % Each job has one task per machine.
set of int: jobs = 1..n_jobs;
set of int: tasks = 1..n_tasks_per_job;

% job_task_machine[j, k] is the machine required by task k of job j.
array [jobs, tasks] of 0..(n_machines-1): job_task_machine;

% job_task_duration[j, k] is the duration of task k of job j.
array [jobs, tasks] of int: job_task_duration;

% minimal/maximal duration : bounds on end time
int: min_duration = 
    max([sum([job_task_duration[i, j] | j in tasks]) | i in jobs]);

int: max_duration = sum([job_task_duration[i, j] | i in jobs, j in tasks]);

%-----------------------------------------------------------------------------%
% Model variables.
%

% The start time of each job task.
array [jobs, tasks] of var 0.. max_duration: job_task_start;

% The finishing time is the time of the last task to complete.
var min_duration..max_duration: t_end;

%-----------------------------------------------------------------------------%
% Constraints.
%

% Sanity check: tasks cannot take a negative amount of time.
constraint
    forall ( j in jobs, k in tasks ) (
        job_task_duration[j, k]  >=  0
    );

% Each job task must complete before the next.
constraint
    forall ( j in jobs, k in 1..(n_tasks_per_job - 1) ) (
        job_task_start[j, k] + job_task_duration[j, k]  <=
            job_task_start[j, k + 1]
    );

% The first job task can start no earlier than time step 0.
constraint
    forall ( j in jobs ) (
        0  <=  job_task_start[j, 1]
    );

% Tasks on the same machine cannot overlap (using disjunctive constraint).
include "globals.mzn";

constraint
    forall (m in 0..n_machines - 1) (
        let {
            array[int] of var 0..max_duration: starts = 
                [ job_task_start[j, k] | j in jobs, k in tasks where job_task_machine[j, k] == m ],
            array[int] of int: durations = 
                [ job_task_duration[j, k] | j in jobs, k in tasks where job_task_machine[j, k] == m ]
        } in
            disjunctive(starts, durations)
    );

% The finishing time must be no earlier than the finishing time of any task.
constraint
    forall ( j in jobs ) (
        job_task_start[j, n_tasks_per_job] +
        job_task_duration[j, n_tasks_per_job]
        <=
        t_end
    );

%-----------------------------------------------------------------------------%
% Objective.
%

solve minimize t_end;

output [
    "job_task_start = ", show(job_task_start), "\n",
    "t_end = ", show(t_end), "\n"
];

%-----------------------------------------------------------------------------%
