import { ChangeEventHandler, FC, useState, useEffect } from "react";
import type { Todo } from "../types/todo";
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
} from "@mui/material";
import { modalInnerStyle } from "../styles/modal";

type Props = {
  todo: Todo;
  onUpdate: (todo: Todo) => void;
  onDelete: (id: number) => void;
};

const TodoItem: FC<Props> = ({ todo, onUpdate, onDelete }) => {
  const [editText, setEditText] = useState("");
  const [isEditing, setIsEditing] = useState(false);

  useEffect(() => {
    setEditText(todo.text);
  }, [todo]);

  const handleCompletedCheckbox: ChangeEventHandler = () => {
    onUpdate({ ...todo, completed: !todo.completed });
  };

  const onCloseEditModal = () => {
    onUpdate({ ...todo, text: editText });
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
        </Box>
      </Modal>
    </Card>
  );
};

export default TodoItem;
