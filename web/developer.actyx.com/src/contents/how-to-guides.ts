/* 
This file contains all categories and their contents of how-to guides.
*/

const createHowTos = () => {
  const howToGuides = [
    {
      category: 'Local Development',
      description:
        'Get your local environment set up; install Actyx and the SDK, start a new project, setup your environment, and debug common errors.',
      contents: [
        {
          title: 'Installing and starting Actyx',
          link: '/docs/how-to-guides/local-development/installing-actyx',
        },
        {
          title: 'Starting a new project',
          link: '/docs/how-to-guides/local-development/starting-a-new-project',
        },
        {
          title: 'Setting up your JS environment',
          link: '/docs/how-to-guides/local-development/setting-up-your-environment',
        },
        {
          title: 'Installing Actyx CLI and Node Manager',
          link: '/docs/how-to-guides/local-development/installing-cli-node-manager',
        },
        {
          title: 'Obtaining your development certificate',
          link: '/docs/how-to-guides/local-development/obtaining-a-development-certificate',
        },
        {
          title: 'Tips & tricks for common development errors',
          link: '/docs/how-to-guides/local-development/common-development-errors',
        },
      ],
    },
    {
      category: 'Process Logic',
      description:
        'Implement processes by writing local twins. Execute these processes and integrate them into the real world through apps.',
      contents: [
        {
          title: 'Publishing to event streams',
          link: '',
        },
        {
          title: 'Subscribing to and querying event streams',
          link: '',
        },
        {
          title: 'Computing a state from events',
          link: '',
        },
        {
          title: 'Automating decision making',
          link: '',
        },
        {
          title: 'Dealing with network partitions',
          link: '',
        },
        {
          title: 'Modelling processes in twins',
          link: '',
        },
        {
          title: 'Transferring twins into code',
          link: '',
        },
      ],
    },
    {
      category: 'Using the SDK to its full potential',
      description:
        'Actyx SDK is your toolbox for implementing processes and automating factory solutions.',
      contents: [
        {
          title: 'SDK-Guide-01',
          link: '',
        },
        {
          title: 'SDK-Guide-02',
          link: '',
        },
        {
          title: 'SDK-Guide-03',
          link: '',
        },
        {
          title: 'SDK-Guide-04',
          link: '',
        },
        {
          title: 'SDK-Guide-05',
          link: '',
        },
        {
          title: 'SDK-Guide-06',
          link: '',
        },
      ],
    },
    {
      category: 'Getting data into and out of Actyx',
      description:
        'Implement processes by writing local twins. Execute these processes and integrate them into the real-world through apps.',
      contents: [
        {
          title: 'Integrating logic with a user interface',
          link: '',
        },
        {
          title: 'Integrating with other software',
          link: '',
        },
        {
          title: 'Integrating with front-end frameworks (react, angular)',
          link: '',
        },
        {
          title: 'Integrating with PLCs',
          link: '',
        },
        {
          title: 'Integrating with ERPs',
          link: '',
        },
        {
          title: 'Integrating with business intelligence / analytics',
          link: '',
        },
      ],
    },
    {
      category: 'Testing with Actyx',
      description:
        'Add unit and end-to-end tests for your twins and apps. Use well established tools such as Jest or Cypress. Set up a CI/CD pipeline.',
      contents: [
        {
          title: 'Designing a testing pipeline',
          link: '',
        },
        {
          title: 'Unit-testing with Jest',
          link: '',
        },
        {
          title: 'Unit-testing with Cypress',
          link: '',
        },
        {
          title: 'Setting up integration testing',
          link: '',
        },
        {
          title: 'Setting up a CI/CD pipeline',
          link: '',
        },
      ],
    },
    {
      category: 'Configuring, Packaging & Deploying',
      description: 'Set up Actyx swarms. Package and deploy apps across nodes.',
      contents: [
        {
          title: 'Packaging front-end apps',
          link: '',
        },
        {
          title: 'Packaging headless apps',
          link: '',
        },
        {
          title: 'Deploying to production',
          link: '',
        },
        {
          title: 'Updating an Actyx solution',
          link: '',
        },
        {
          title: 'Turning your Actyx nodes into a swarm',
          link: '',
        },
        {
          title: 'Setting up a bootstrap node',
          link: '',
        },
      ],
    },
    {
      category: 'Monitoring & Debugging',
      description:
        'Debug your code locally on your machine or remote-monitor installations with 3rd party mobile device management tools.',
      contents: [
        {
          title: 'Accessing node logs',
          link: '',
        },
        {
          title: 'Publishing and accessing app logs',
          link: '',
        },
        {
          title: "Using the node's connectivity status for debugging",
          link: '',
        },
        {
          title: 'Using 3rd party mobile device management tools',
          link: '',
        },
        {
          title: 'Accelerating your workflow with bash',
          link: '',
        },
      ],
    },
    {
      category: 'Common use-cases',
      description:
        'Implement common use-cases for Actyx. including dashboards, ERP integrations, control logic and tool connectivity.',
      contents: [
        {
          title: 'Showing machine data on a dashboard',
          link: '',
        },
        {
          title: 'Display ERP orders on tablets',
          link: '',
        },
        {
          title: 'Controlling AGVs delivering materials',
          link: '',
        },
        {
          title: 'Parameterise an assembly tool',
          link: '',
        },
      ],
    },
  ]
  return howToGuides
}

const howToGuides = createHowTos()
export default howToGuides
