import React from "react";
import styled from "styled-components";
import defaults from "../components/defaults";

const StrapWrapper = styled.div`
  font-family: "system-ui", "-apple-system", "Helvetica", sans-serif;
  font-size: ${defaults.fontsizes.title};
  font-weight: ${defaults.fontWeights.regular};
  color: ${defaults.colors.primary};
  margin-top: 7px;
  margin-bottom: -92px;
`;

type Props = Readonly<{
  strap: string;
}>;

/*
Important: When using a strap, specify `hide_title: true` in the meta section of the markdown.
Then use the strap above an #h1 headline which will be the title of the document.

Example:

<Strap strap={"Tutorial"} />

# Building a chat app with Actyx Pond

*/
export const Strap = ({ strap }: Props) => <StrapWrapper>{strap}</StrapWrapper>;
