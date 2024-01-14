import { useState, FC } from "react";
import { NewTodoPayload, Label } from "../types/todo";
import {
  Box,
  Button,
  TextField,
  Paper,
  Grid,
  FormControlLabel,
  Checkbox,
  Stack,
  Modal,
  Chip,
} from "@mui/material";
import { modalInnerStyle } from "../styles/modal";
import { toggleLabels } from "../lib/toggleLabels";

type Props = {
  onSubmit: (payload: NewTodoPayload) => void;
  labels: Label[];
};

const TodoForm: FC<Props> = ({ onSubmit, labels }) => {
  const [todoText, setTodoText] = useState("");
  const [labelList, setLabelList] = useState<Label[]>([]);
  const [openLabelModal, setOpenLabelModal] = useState(false);

  const addTodoHandler = async () => {
    if (!todoText) return;
    onSubmit({ text: todoText, label_ids: labelList.map((label) => label.id) });
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
          <Grid item xs={12}>
            <Stack direction="row" spacing={1}>
              {labelList.map((label) => (
                <Chip key={label.id} label={label.name} />
              ))}
              <Button onClick={() => setOpenLabelModal(true)}>add label</Button>
            </Stack>
          </Grid>
          <Grid item xs={3} xl={7}>
            <Button
              onClick={() => setOpenLabelModal(true)}
              fullWidth
              color="secondary"
            >
              select label
            </Button>
          </Grid>
          <Grid item xs={6} />
          <Grid item xs={3}>
            <Button onClick={addTodoHandler} fullWidth>
              add todo
            </Button>
          </Grid>
          <Modal open={openLabelModal} onClose={() => setOpenLabelModal(false)}>
            <Box sx={modalInnerStyle}>
              <Stack>
                {labels.map((label) => (
                  <FormControlLabel
                    key={label.id}
                    control={<Checkbox checked={labelList.includes(label)} />}
                    label={label.name}
                    onChange={() =>
                      setLabelList((prev) => toggleLabels(prev, label))
                    }
                  />
                ))}
              </Stack>
            </Box>
          </Modal>
        </Grid>
      </Box>
    </Paper>
  );
};

export default TodoForm;
