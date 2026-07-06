

export const Logo = ({ size = 40, dark }) => {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap: '12px', minWidth: '180px' }}>
      <div style={{ position: 'relative', width: size, height: size, flexShrink: 0 }}>
        <img 
          src="/black_cat.png" 
          alt="Logo" 
          style={{ 
            position: 'absolute',
            inset: 0,
            width: '100%',
            height: '100%',
            objectFit: 'contain',
            opacity: dark ? 0 : 1,
            transition: 'opacity 0.3s ease'
          }} 
        />
        <img 
          src="/white_cat.png" 
          alt="Logo" 
          style={{ 
            position: 'absolute',
            inset: 0,
            width: '100%',
            height: '100%',
            objectFit: 'contain',
            opacity: dark ? 1 : 0,
            transition: 'opacity 0.3s ease'
          }} 
        />
      </div>
      <span style={{
        fontFamily: "'Google Sans', sans-serif",
        fontSize: '22px',
        color: 'var(--gd-on-surface-variant)',
        fontWeight: 400,
        letterSpacing: '-0.5px'
      }}>
        Drive
      </span>
    </div>
  );
};
