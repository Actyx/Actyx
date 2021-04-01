/* 
This file contains all categories and their contents of how-to guides.
*/

const createTutorials = () => {
  const tutorials = [
    {
      title: 'Quickstart Guide',
      description: 'Write your first Hello World on the Actyx low-code platform.',
      link: 'quickstart',
    },
    {
      title: 'Chat App Tutorial',
      description: 'Write a distributed chat application in a few simple steps.',
      link: 'tutorial',
    },
    {
      title: 'Factory Tutorial',
      description: 'See how a real factory solution could look like when built on Actyx.',
      link: 'advanced-tutorial/introduction',
    },
  ]
  return tutorials
}

const tutorials = createTutorials()
export default tutorials
