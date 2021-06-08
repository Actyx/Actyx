const createTheme = () => {
  const theme = {
    fontsizes: {
      hero: '26px',
      title: '20px',
      subheadline: '17px',
      body: '16px',
      small: '12px',
      medium: '14px',
    },
    fontWeights: {
      thin: 400,
      regular: 500,
      bold: 600,
    },
    fontfamily: "'system-ui', '-apple-system' , 'Helvetica', sans-serif",
    viewport: {
      small: 450,
      medium: 800,
      large: 1440,
    },
    colors: {
      primary: '#1998ff',
      green: '#15be53',
      purple: '#635bff',
      blue: '#369aff',
      orange: '#ff9933',
      darkgray: '#303c4b',
      gray: '#f5f5f5',
      lightgray: '#ebedf0',
      white: '#ffffff',
      black: '#000000',
    },
  }
  return theme
}

const theme = createTheme()
export default theme
