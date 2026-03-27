import { Composition } from 'remotion';
import { DictationDemo } from './DictationDemo';
import { PrivacyComparison } from './PrivacyComparison';
import { CrossPlatform } from './CrossPlatform';

export const Root: React.FC = () => {
  return (
    <>
      <Composition
        id="DictationDemo"
        component={DictationDemo}
        durationInFrames={300}
        fps={30}
        width={1920}
        height={1080}
      />
      <Composition
        id="PrivacyComparison"
        component={PrivacyComparison}
        durationInFrames={300}
        fps={30}
        width={1920}
        height={1080}
      />
      <Composition
        id="CrossPlatform"
        component={CrossPlatform}
        durationInFrames={300}
        fps={30}
        width={1920}
        height={1080}
      />
    </>
  );
};
