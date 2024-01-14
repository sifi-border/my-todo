import { ChangeEventHandler, FC, useState, useEffect } from "react";
import type { Todo, Label, UpdateTodoPayload } from "../types/todo";
import {
  Card,
  Checkbox,
  Stack,
  Typography,
  Button,
  Grid,
  Modal,
  Box,
  TextField,
  Chip,
  FormControlLabel,
} from "@mui/material";
import { modalInnerStyle } from "../styles/modal";
import { toggleLabels } from "../lib/toggleLabels";

type Props = {
  todo: Todo;
  onUpdate: (todo: UpdateTodoPayload) => void;
  onDelete: (id: number) => void;
  labels: Label[];
};

const TodoItem: FC<Props> = ({ todo, onUpdate, onDelete, labels }) => {
  const [editText, setEditText] = useState("");
  const [isEditing, setIsEditing] = useState(false);
  const [labelList, setLabelList] = useState<Label[]>([]);

  useEffect(() => {
    setEditText(todo.text);
    setLabelList(todo.labels);
  }, [todo, isEditing]);

  const handleCompletedCheckbox: ChangeEventHandler = () => {
    onUpdate({
      ...todo,
      completed: !todo.completed,
      label_ids: todo.labels.map((label) => label.id),
    });
  };

  const onCloseEditModal = () => {
    onUpdate({
      id: todo.id,
      text: editText,
      completed: todo.completed,
      label_ids: labelList.map((label) => label.id),
    });
    setIsEditing(false);
  };

  const handleDeleteButton = () => {
    onDelete(todo.id);
  };

  return (
    <Card sx={{ p: 1 }}>
      <Grid container spacing={2} alignItems="center">
        <Grid item xs={1}>
          <Checkbox
            checked={todo.completed}
            onChange={handleCompletedCheckbox}
          />
        </Grid>
        <Grid item xs={8}>
          <Stack spacing={1}>
            <Typography variant="caption" fontSize={16}>
              {todo.text}
            </Typography>
            <Stack direction="row" spacing={1}>
              {todo.labels?.map((label) => (
                <Chip key={label.id} label={label.name} size="small" />
              ))}
            </Stack>
          </Stack>
        </Grid>
        <Grid item xs={2}>
          <Stack direction="row" spacing={1}>
            <Button onClick={() => setIsEditing(true)} color="info">
              edit
            </Button>
            <Button onClick={handleDeleteButton} color="error">
              delete
            </Button>
          </Stack>
        </Grid>
      </Grid>
      <Modal open={isEditing} onClose={onCloseEditModal}>
        <Box sx={modalInnerStyle}>
          <Stack spacing={2}>
            <TextField
              size="small"
              label="todo text"
              defaultValue={todo.text}
              onChange={(e) => setEditText(e.target.value)}
            />
          </Stack>
          <Stack>
            <Typography variant="subtitle1">Labels</Typography>
            {labels.map((label) => (
              <FormControlLabel
                key={label.id}
                control={
                  <Checkbox
                    defaultChecked={todo.labels.some(
                      (todoLabel) => todoLabel.id === label.id
                    )}
                  />
                }
                label={label.name}
                onChange={() =>
                  setLabelList((prev) => toggleLabels(prev, label))
                }
              />
            ))}
          </Stack>
        </Box>
      </Modal>
    </Card>
  );
};

export default TodoItem;
