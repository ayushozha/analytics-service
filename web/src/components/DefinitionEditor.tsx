import { useState } from 'react';
import { parseTags, saveDefinition } from '../lib/definitions';
import type { DefinitionKind, DefinitionMeta } from '../lib/types';

/** Edit a definition's description, tags and flags; calls `onSaved` after a successful save. */
export function DefinitionEditor({ kind, name, meta, onSaved, onClose }: {
  kind: DefinitionKind;
  name: string;
  meta: DefinitionMeta;
  onSaved: () => void;
  onClose: () => void;
}) {
  const [description, setDescription] = useState(meta.description ?? '');
  const [tags, setTags] = useState(meta.tags.join(', '));
  const [verified, setVerified] = useState(meta.verified);
  const [hidden, setHidden] = useState(meta.hidden);
  const [pii, setPii] = useState(meta.is_pii);
  const [state, setState] = useState<'idle' | 'saving' | 'error'>('idle');
  const [error, setError] = useState('');

  const save = async () => {
    setState('saving');
    try {
      await saveDefinition(kind, name, { description, tags: parseTags(tags), verified, hidden, is_pii: pii });
      setState('idle');
      onSaved();
    } catch (e) {
      setError((e as Error).message);
      setState('error');
    }
  };

  const check = (label: string, value: boolean, set: (v: boolean) => void, hint: string) => (
    <label className="toolbar" title={hint}>
      <input type="checkbox" checked={value} onChange={(e) => set(e.target.checked)} style={{ accentColor: 'var(--accent)' }} />
      {label}
    </label>
  );

  return (
    <section className="card editor" aria-label={`Edit ${name}`}>
      <div className="card-head">
        <h2 className="mono" style={{ fontSize: 14 }}>{name}</h2>
        <button className="btn btn-sm" aria-label="Close editor" onClick={onClose}>✕</button>
      </div>
      <label className="field">
        <span>Description</span>
        <textarea className="input textarea" rows={3} value={description} placeholder="What does this mean and when is it sent?"
          onChange={(e) => setDescription(e.target.value)} />
      </label>
      <label className="field">
        <span>Tags</span>
        <input className="input" value={tags} placeholder="growth, activation" onChange={(e) => setTags(e.target.value)} />
      </label>
      <div className="toolbar" style={{ gap: 16 }}>
        {check('Verified', verified, setVerified, 'Marks this as reviewed and trustworthy')}
        {check('Hidden', hidden, setHidden, 'Hide from pickers and lists')}
        {kind !== 'event' && check('Contains PII', pii, setPii, 'Personal data')}
      </div>
      {state === 'error' && <div className="error" role="alert" style={{ padding: 0, textAlign: 'left' }}>{error}</div>}
      <div className="toolbar">
        <button className="btn btn-primary" disabled={state === 'saving'} onClick={save}>{state === 'saving' ? 'Saving…' : 'Save'}</button>
        <button className="btn" onClick={onClose}>Cancel</button>
      </div>
    </section>
  );
}

export function DefinitionBadges({ meta }: { meta: DefinitionMeta }) {
  return (
    <>
      {meta.verified && <span className="badge badge-verified" title="Verified">✓ Verified</span>}
      {meta.hidden && <span className="badge">Hidden</span>}
      {meta.is_pii && <span className="badge badge-pii">PII</span>}
    </>
  );
}
