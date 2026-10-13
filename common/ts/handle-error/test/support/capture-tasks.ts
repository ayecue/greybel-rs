export type Task = () => void;

export function captureTasks(): {
  tasks: Task[];
  scheduler: (task: Task) => void;
} {
  const tasks: Task[] = [];
  return { tasks, scheduler: (task) => void tasks.push(task) };
}

export function thrownBy(task: Task | undefined): unknown {
  try {
    task?.();
  } catch (error) {
    return error;
  }
  return undefined;
}
