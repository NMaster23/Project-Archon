import { json } from '@codemirror/lang-json';
import { githubDark } from '@uiw/codemirror-theme-github';
import CodeMirror from '@uiw/react-codemirror';
import { useEffect, useState } from 'react';

export default function ConfigEditor({config, onChange}: {config: any, onChange: (newConfig: any) => void;}) {
    const [code, setCode] = useState(() => JSON.stringify(config, null, 2));
    useEffect(() => {
        try {
            const currentParsed = JSON.parse(code);
            if (JSON.stringify(currentParsed) !== JSON.stringify(config)) {
                setCode(JSON.stringify(config, null, 2));
            }
        } catch (e) {
            setCode(JSON.stringify(config, null, 2));
        }
    }, [config]);
    const editorChange = (value: string | undefined) => {
        const newVal = value ??  ''
        setCode(newVal);
        try {
            const parsed = JSON.parse(newVal);
            onChange(parsed);
        } catch (e) {
            return
        }
    }
    return (
        <CodeMirror
            className='h-full rounded-xl overflow-hidden'
            theme={githubDark}
            value={code}
            width='100%'
            height='100%'
            extensions={[json()]}
            onChange={editorChange} 
        />
    );
}