import React from 'react';
import ReactDOM from 'react-dom';
import './index.css';
import App from './App';
import { Pond } from '@actyx-contrib/react-pond';


function Waiting() {
  return (
    <div className="loading">
      Waiting for connection to Actyx on localhost — is it running?
    </div>
  )
}

function onError(err) {
  console.warn('Could not connect to Actyx. Retrying in 2s', err)
  setTimeout(() => window.location.reload(), 2000)
}

const manifest = {
  appId: 'com.example.todo-react-actyx',
  displayName: 'Collaborative Todo List using Actyx',
  version: '1.0.0'
}

ReactDOM.render(
  <React.StrictMode>
    <Pond loadComponent={<Waiting/>} onError={onError} manifest={manifest}>
      <App />
    </Pond>
  </React.StrictMode>,
  document.getElementById('root')
);
