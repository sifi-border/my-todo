export type Todo = {
  id: number;
  text: string;
  completed: boolean;
  labels: Label[];
};

export type NewTodoPayload = {
  text: string;
  label_ids: number[];
};

export type UpdateTodoPayload = {
  id: number;
  text?: string;
  completed?: boolean;
  label_ids?: number[];
};

export type Label = {
  id: number;
  name: string;
};

export type NewLabelPayload = {
  name: string;
};
