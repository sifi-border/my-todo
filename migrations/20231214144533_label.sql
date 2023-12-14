CREATE TABLE labels (
    id serial PRIMARY KEY,
    name text NOT NULL
);
CREATE TABLE todo_labels (
    id SERIAL PRIMARY KEY,
    todo_id integer NOT NULL REFERENCES todos(id) DEFERRABLE INITIALLY DEFERRED,
    label_id integer NOT NULL REFERENCES labels(id) DEFERRABLE INITIALLY DEFERRED
);
