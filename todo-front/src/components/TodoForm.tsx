import { useState, FC } from "react";
import { NewTodoPayload } from "../types/todo";
import { Box, Button, TextField, Paper, Grid } from "@mui/material";

type Props = {
  onSubmit: (payload: NewTodoPayload) => void;
};

const TodoForm: FC<Props> = ({ onSubmit }) => {
  const [todoText, setTodoText] = useState("");

  const addTodoHandler = async () => {
    if (!todoText) return;
    onSubmit({ text: todoText });
    setTodoText("");
  };

  return (
    <Paper elevation={2}>
      <Box sx={{ p: 2 }}>
        <Grid container rowSpacing={2} columnSpacing={5}>
          <Grid item xs={12}>
            <TextField
              label="new todo text"
              variant="filled"
              value={todoText}
              onChange={(e) => setTodoText(e.target.value)}
              fullWidth
            />
          </Grid>
          <Grid item xs={9} />
          <Grid item xs={3}>
            <Button onClick={addTodoHandler} fullWidth>
              add todo
            </Button>
          </Grid>
        </Grid>
      </Box>
    </Paper>
  );
};

export default TodoForm;
