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
      title: 'To-Do App Example Project',
      description: 'A to-do app built on the Local-First cooperation paradigm.',
      link: 'https://github.com/actyx-contrib/todo-react',
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
