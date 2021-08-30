import React, { useEffect } from "react"
import { Button } from "../components/basics"
import { shutdownApp } from "../util"
import { FatalError } from "../../common/ipc"
import { safeErrorToStr } from "../../common/util"
import { useAnalytics } from "../analytics"
const Screen: React.FC<{ error: FatalError }> = ({ error }) => {
  const analytics = useAnalytics()
  useEffect(() => {
    if (analytics) {
      analytics.viewedScreen("Fatal Error")
      analytics.gotFatalError(error)
    }
  }, [analytics, error])
  const { details, shortMessage } = error
  const safeDetails = safeErrorToStr(details)
  return (
    <div className="h-full w-full flex justify-center items-center">
      <div className="text-center p-6 px-16">
        <p className="pb-6 text-gray-400 text-sm uppercase font-medium">
          Fatal error
        </p>
        <p className="text-red-500">{shortMessage}</p>
        {safeDetails && <p className="text-sm mt-2">{safeDetails}</p>}
        <p className="text-gray-500 mt-8 ">
          Please exit and restart the Actyx Node Manager.
        </p>
        <Button className="mt-4" onClick={shutdownApp}>
          Exit
        </Button>
      </div>
    </div>
  )
}

export default Screen
