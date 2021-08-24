import { SimpleCanvas } from "../components/SimpleCanvas";
import React from "react";
import { Layout } from "../components/Layout";
import { useStore } from "../store";
import { StoreStateKey } from "../store/types";

const Analytics = () => {
  const store = useStore();
  const checked =
    store.key === StoreStateKey.Loaded && store.data.analytics.disabled;
  const onChange = (isChecked: boolean) => {
    console.log(`onChange(isChecked: ${isChecked})`);
    if (store.key === StoreStateKey.Loaded) {
      store.actions.updateAndReload({
        ...store.data,
        analytics: {
          ...store.data.analytics,
          disabled: isChecked,
        },
      });
    }
  };
  return (
    <label className="inline-flex items-center p-1">
      <input
        type="checkbox"
        checked={checked}
        onChange={(event) => onChange(event.target.checked)}
      />
      <span className="ml-2">
        Disable anonymous aggregate user behaviour analytics
      </span>
    </label>
  );
};

const Screen: React.FC<{}> = () => {
  return (
    <Layout title="Preferences">
      <SimpleCanvas>
        <div className="flex flex-col flex-grow flex-shrink">
          <p className="text-gray-400 pb-3 flex-grow-0 flex-shrink-0">
            Configure the Node Manager to fit your workflow.
          </p>
          <Analytics />
        </div>
      </SimpleCanvas>
    </Layout>
  );
};

export default Screen;
