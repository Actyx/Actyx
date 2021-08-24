import { StoreData as Data } from "../../common/types";

export const enum StoreStateKey {
  Loaded = "Loaded",
  LoadingOrSaving = "LoadingOrSaving",
  Initial = "Initial",
}

interface LoadedActions {
  updateAndReload: (new_: Data) => void;
  reload: () => void;
}

interface Loaded {
  key: StoreStateKey.Loaded;
  data: Data;
  actions: LoadedActions;
}
interface LoadingOrSaving {
  key: StoreStateKey.LoadingOrSaving;
}

interface Initial {
  key: StoreStateKey.Initial;
}

// Any error will actually be fatal for simplicity
export type StoreState = Initial | Loaded | LoadingOrSaving;

export const enum StoreActionKey {
  HasLoaded = "HasLoaded",
  LoadOrSave = " LoadOrSave",
}

interface HasLoaded {
  key: StoreActionKey.HasLoaded;
  data: Data;
  actions: LoadedActions;
}

interface LoadOrSave {
  key: StoreActionKey.LoadOrSave;
}

export type StoreAction = HasLoaded | LoadOrSave;

export type Dispatch = (action: StoreAction) => void;
