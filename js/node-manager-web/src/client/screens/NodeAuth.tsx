import { SimpleCanvas } from "../components/SimpleCanvas";
import React, { useEffect, useReducer, useState } from "react";
import { Layout } from "../components/Layout";
import { Button, SimpleInput } from "../components/basics";
import { useAppState } from "../app-state";
import { isNone } from "fp-ts/lib/Option";
import {
  getFolderFromUser,
  Wizard,
  WizardFailure,
  WizardSuccess,
  WizardInput,
} from "../util";
import { Either, left, right } from "fp-ts/lib/Either";
import { FatalError } from "../../common/ipc";
import { safeErrorToStr } from "../../common/util";
import { useStore } from "client/store";

const Screen = () => {
  const {
    actions: { createUserKeyPair },
  } = useAppState();

  const execute = ({
    location,
  }: Input): Promise<Either<FatalError, Success>> => {
    return createUserKeyPair()
      .then((resp) => right(resp))
      .catch((e) => left(e));
  };

  return (
    <Layout title="Node Authentication">
      <SimpleCanvas>
        <Wizard
          failure={Failed}
          success={Success}
          input={Initial}
          execute={execute}
        />
      </SimpleCanvas>
    </Layout>
  );
};

interface Input {
  location:
    | undefined
    | {
        folder: string;
        name: string;
      };
}
interface Success {
  privateKey: string;
}

// FIXME: download key
const Initial: WizardInput<Input> = ({ execute, executing }) => {
  const [folder, setFolder] = useState("");
  const [name, setName] = useState("");

  const selectFolder = async () => {
    const folder = await getFolderFromUser();
    if (isNone(folder)) {
      console.log(`did not get folder`);
      return;
    }
    console.log(`got folder ${folder.value}`);
    setFolder(folder.value);
  };

  const doExecute = () => {
    if (folder === "" || name === "") {
      return;
    }
    execute({ location: { folder, name } });
  };

  const doExecuteDefault = () => {
    execute({ location: undefined });
  };

  return (
    <>
      <p>Generate a user key pair to authenticate yourself to Actyx nodes.</p>
      <div>
        <SimpleInput
          className="mt-4"
          label="Name of key pair"
          placeholder="Name"
          setValue={setName}
          value={name}
          disabled={executing}
        />
        <SimpleInput
          className="mt-4"
          label="Directory to save key pair in"
          placeholder="Select directory to save key pair in"
          setValue={setFolder}
          value={folder}
          disabled={true}
          button={{
            text: "Select directory",
            onClick: selectFolder,
            disabled: executing,
          }}
        />
        <div className="flex mt-8">
          <Button
            onClick={doExecute}
            disabled={folder === "" || name === ""}
            working={executing}
          >
            Create key pair
          </Button>
          <Button
            className="ml-3"
            onClick={doExecuteDefault}
            disabled={executing}
            working={executing}
          >
            Create default
          </Button>
        </div>
      </div>
    </>
  );
};

const Success: WizardSuccess<Success> = ({
  restart,
  result: { privateKey },
}) => (
  <>
    <p className="mb-2">Successfully generated user key pair.</p>
    <p>You can now use this key pair to interact with Actyx nodes.</p>
    <div>
      <SimpleInput
        className="mt-4"
        label="Your private key is"
        value={privateKey}
        disabled
        inputClassName="text-sm text-gray-600"
      />
      <Button className="mt-8" onClick={restart}>
        Back
      </Button>
    </div>
  </>
);

const Failed: WizardFailure<FatalError> = ({
  restart,
  reason: { shortMessage, details },
}) => (
  <>
    <p className="text-red-500 font-medium mb-2">
      Error creating user key pair
    </p>
    <p className="mb-2">{shortMessage}</p>
    {details && <p>{safeErrorToStr(details)}</p>}
    <Button className="mt-8" onClick={restart}>
      Back
    </Button>
  </>
);

export default Screen;
