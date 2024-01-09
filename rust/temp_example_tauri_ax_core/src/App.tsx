import { useState } from "react";
import { invoke } from "@tauri-apps/api/tauri";
import "./App.css";

function App() {
  const [result, setResult] = useState(null);

  const queryAllEvents = () => {
    invoke("query_all_events")
      .then((res) =>
        setResult(
          (res as any).map((item: any) => JSON.stringify(item)).join("\n")
        )
      )
      .catch((e) => console.error(e));
  };

  return (
    <div className="container">
      <h1>Ax Run as Embedded Something!</h1>

      <form
        className="row"
        onSubmit={(e) => {
          e.preventDefault();
          queryAllEvents();
        }}
      >
        <button type="submit">queryAllEvents</button>
      </form>

      {result && <pre>{result}</pre>}
    </div>
  );
}

export default App;
