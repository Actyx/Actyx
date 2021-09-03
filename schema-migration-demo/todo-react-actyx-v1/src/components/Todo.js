import { useFishFn } from "@actyx-contrib/react-pond";
import React, { useEffect, useRef, useState } from "react";
import { taskDeleted, taskEdited, taskStatus, taskDetailsFish } from "../model";


function usePrevious(value) {
  const ref = useRef();
  useEffect(() => {
    ref.current = value;
  });
  return ref.current;
}

export default function Todo(props) {
  const task = useFishFn(taskDetailsFish, props.id);
  const taskState = task === undefined ? {} : task.state;

  const [isEditing, setEditing] = useState(false);
  const [newName, setNewName] = useState('');

  const editFieldRef = useRef(null);
  const editButtonRef = useRef(null);

  const wasEditing = usePrevious(isEditing);

  function handleChange(e) {
    setNewName(e.target.value);
  }

  function handleSubmit(e) {
    e.preventDefault();
    if (!newName.trim()) {
      return;
    }
    task.run((state, enqueue) => enqueue(...taskEdited(state.id, newName)));
    setNewName("");
    setEditing(false);
  }

  const editingTemplate = (
    <form className="stack-small" onSubmit={handleSubmit}>
      <div className="form-group">
        <label className="todo-label" htmlFor={taskState.id}>
          New name for {taskState.name}
        </label>
        <input
          id={taskState.id}
          className="todo-text"
          type="text"
          value={newName || taskState.name}
          onChange={handleChange}
          ref={editFieldRef}
        />
      </div>
      <div className="btn-group">

        <button
          type="button"
          className="btn todo-cancel"
          onClick={() => setEditing(false)}
        >
          Cancel
          <span className="visually-hidden">renaming {taskState.name}</span>
        </button>
        <button type="submit" className="btn btn__primary todo-edit">
          Save
          <span className="visually-hidden">new name for {taskState.name}</span>
        </button>
      </div>
    </form>
  );

  const viewTemplate = (
    <div className="stack-small">
      <div className="c-cb">
          <input
            id={taskState.id}
            type="checkbox"
            checked={taskState.completed}
            onChange={() => task.run((state, enqueue) => enqueue(...taskStatus(state.id, !state.completed)))}
          />
          <label className="todo-label" htmlFor={taskState.id}>
            {taskState.name}
          </label>
        </div>
        <div className="btn-group">
        <button
          type="button"
          className="btn"
          onClick={() => setEditing(true)}
          ref={editButtonRef}
          >
            Edit <span className="visually-hidden">{taskState.name}</span>
          </button>
          <button
            type="button"
            className="btn btn__danger"
            onClick={() => task.run((state, enqueue) => enqueue(...taskDeleted(state.id)))}
          >
            Delete <span className="visually-hidden">{taskState.name}</span>
          </button>
        </div>
    </div>
  );


  useEffect(() => {
    if (!wasEditing && isEditing) {
      editFieldRef.current.focus();
    }
    if (wasEditing && !isEditing) {
      editButtonRef.current.focus();
    }
  }, [wasEditing, isEditing]);

  if (task === undefined || !props.show(taskState)) {
    return null
  }

  return <li className="todo">{isEditing ? editingTemplate : viewTemplate}</li>;
}
