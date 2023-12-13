import "modern-css-reset";
import { useState, useEffect, FC } from "react";
import { NewTodoPayload, Todo } from "./types/todo";
import { ThemeProvider, createTheme } from "@mui/material/styles";
import { Box, Stack, Typography } from "@mui/material";
import TodoForm from "./components/TodoForm";
import TodoList from "./components/TodoList";
import {
  addTodoItem,
  getTodoItems,
  updateTodoItem,
  deleteTodoItem,
} from "./lib/api/todo";

const TodoApp: FC = () => {
  const [todos, setTodos] = useState<Todo[]>([]);

  const onSubmit = async (payload: NewTodoPayload) => {
    if (!payload.text) return;
    await addTodoItem(payload);
    const newTodos = await getTodoItems();
    setTodos(newTodos);
  };

  const onUpdate = async (updateTodo: Todo) => {
    await updateTodoItem(updateTodo);
    const newTodos = await getTodoItems();
    setTodos(newTodos);
  };

  const onDelete = async (id: number) => {
    await deleteTodoItem(id);
    const newTodos = await getTodoItems();
    setTodos(newTodos);
  };

  useEffect(() => {
    (async () => {
      const todos = await getTodoItems();
      setTodos(todos);
    })();
  }, []);

  return (
    <>
      <Box
        sx={{
          backgroundColor: "white",
          borderBottom: "1px solid gray",
          display: "flex",
          alignItems: "center",
          position: "fixed",
          top: 0,
          p: 2,
          width: "100%",
          height: 80,
          zIndex: 3,
        }}
      >
        <Typography variant="h1">Todo App</Typography>
      </Box>
      <Box
        sx={{
          display: "flex",
          justifyContent: "center",
          p: 5,
          mt: 10,
        }}
      >
        <Box maxWidth={700} width="100%">
          <Stack spacing={5}>
            <TodoForm onSubmit={onSubmit} />
            <TodoList todos={todos} onUpdate={onUpdate} onDelete={onDelete} />
          </Stack>
        </Box>
      </Box>
    </>
  );
};

const theme = createTheme({
  typography: {
    h1: {
      fontSize: 30,
    },
    h2: {
      fontSize: 20,
    },
  },
});

const App: FC = () => {
  return (
    <ThemeProvider theme={theme}>
      <TodoApp />
    </ThemeProvider>
  );
};

export default App;
