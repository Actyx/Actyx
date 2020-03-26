
## Documentation structure
ActyxOS
- Getting Started
  - Installation
    - Install ActyxOS
    - Download the CLI
    - Start the node
    - Deploy an app
      - Clone from Github
- Guides
  - Creating swarms
  - Joining swarms
  - Building apps
    - App Types
    - App Manifests
    - Packaging Apps
  - Deploying apps
  - Running apps
  - Event streams
  - Logging
- Advanced guides

  - App Runtimes
    - Compatibility with hosts
    - App Lifecyle
    - WebView Runtime
    - Docker Runtime
  - App Settings
    - Settings Schema
    - Accessing Settings
  - Node Lifecycle
  - Node Settings
  - ActyxOS on Android
    - Installation
    - The ActyxOS UI
    - Troubleshooting
  - ActyxOS on Docker
    - Installation
    - Troubleshooting
  - Using WorkspaceOne
  - Using Balena
- API reference
  - Event Service API
  - Console Service API
  - 

















Actyx Pond
- Getting Started
  - Installation
    - npm install
    - Connect with your node
  - Tutorial 



 > TODO: check if outbound 4001 works.

  Quick Start
   - git clone http://github.com/actyx/quickstart
   - docker run actyx/os
   - npm run ax:apps:sample:start firstInstance
   - npm run ax:apps:sample:start secondInstance
   - Download Actyx CLI
   - ax apps package webview-sample/manifest.yml 
   - ax apps deploy --local ... (Docker | Android)
   - ax settings set --local com.actyx.os @sample-node-settings.yml
   - ax apps start --local ... (Docker | Android)
    
