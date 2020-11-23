import React, { useEffect, useState } from "react";
import useDocusaurusContext from "@docusaurus/useDocusaurusContext";
//import usePrismTheme from"@theme/hooks/usePrismThem";
//import Playground from "@theme/Playground";
//import ReactLiveScope from "@theme/ReactLiveScope";
import Layout from "@theme-init/Layout";
import Cookies from "js-cookie";

export const setDataLayerVariable = (name, value) => {
  if (typeof window !== `undefined` && window.dataLayer) {
    const x = {};
    x[name] = value;
    window.dataLayer.push(x);
  } else {
    console.error(
      `Unable to set data layer variable since window or window.dataLayer not found or undefined.`
    );
  }
};

// Pass in an object
export const setDataLayerVariables = (values) => {
  if (typeof window !== `undefined` && window.dataLayer) {
    window.dataLayer.push(values);
  } else {
    console.error(
      `Unable to set data layer variable since window or window.dataLayer not found or undefined.`
    );
  }
};

const INTERNAL_USER = "ax-internal-user";

export const setInternalUserCookie = () => {
  console.debug(`Setting the ${INTERNAL_USER} cookie to 'true'`);
  Cookies.set(INTERNAL_USER, "true");
};

export const unsetInternalUserCookie = () => {
  console.debug(`Removing the ${INTERNAL_USER} cookie`);
  Cookies.remove(INTERNAL_USER);
};

export const internalUserCookieIsSet = () => {
  return Cookies.get(INTERNAL_USER) === "true";
};

export const toogleInternalUserCookieState = () => {
  if (internalUserCookieIsSet()) {
    unsetInternalUserCookie();
  } else {
    setInternalUserCookie();
  }
};

const markAsInternalUser = () => {
  console.log(`Marking as internal user`);
  setInternalUserCookie();
  alert(`Marked as internal user`);
};
const unmarkAsInternalUser = () => {
  console.log(`Unmarking as internal user`);
  unsetInternalUserCookie();
  alert(`Unmarked as internal user`);
};
const isMarkedAsInternalUser = () => {
  return internalUserCookieIsSet();
};
const toggleMarkAsInternalUser = () => {
  if (isMarkedAsInternalUser()) {
    unmarkAsInternalUser();
  } else {
    markAsInternalUser();
  }
};

const NUM_SECRET_CLICKS = 10;

const withAnalytics = (Component) => {
  const WrappedComponent = (props) => {
    //const prismTheme = usePrismTheme();

    const [_numClicks, setNumClicks] = useState(0);
    const [isInternalUser, setIsInternalUser] = useState(false);
    useEffect(() => {
      const _isMarkedAsInternalUser = isMarkedAsInternalUser();
      setDataLayerVariables({
        internalUser: _isMarkedAsInternalUser,
        path: window.location.pathname,
        normPath: window.location.pathname,
        locale: "en",
      });
      setIsInternalUser(_isMarkedAsInternalUser);
    }, []);

    const onSecretClick = () => {
      setNumClicks((c) => {
        if (c + 1 >= NUM_SECRET_CLICKS) {
          toggleMarkAsInternalUser();
          setIsInternalUser(true);
          return 0;
        }
        return c + 1;
      });
    };

    return (
      <>
        {isInternalUser && (
          <div
            onClick={onSecretClick}
            style={{
              height: "4px",
              backgroundColor: "#e50cff",
            }}
          />
        )}
        <Component {...props} />
        <div
          onClick={onSecretClick}
          style={{
            height: "10px",
            backgroundColor: "#f5f6f7",
          }}
        />
      </>
    );
  };

  return WrappedComponent;
};

export default withAnalytics(Layout);
