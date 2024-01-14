import { Label } from "../types/todo";

export const toggleLabels = (labels: Label[], label: Label) =>
  labels.find(({ id }) => id === label.id)
    ? labels.filter(({ id }) => id !== label.id)
    : [...labels, label];
