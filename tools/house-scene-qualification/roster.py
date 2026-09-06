"""Bounded source-list census projection; no script/condition execution."""
from census import require, u, sha

# Source record addresses and RE dispositions, not runtime slots/initialization.
# Codes: P=player, R=admitted resident, H=hidden interaction/controller,
# X=rejected/departed on the fresh path, E=scene effect, V=phase-dependent visual,
# C=compact service, T=automatic trailing-text service.
RECORDS={
  0xB:[(0x8b7e,'FD','P'),(0x8b96,'O','R'),(0x8ba0,'FD','H'),(0x8ba7,'FD','H'),(0x8bae,'O','X'),(0x8bb8,'FF','T')],
  0xC:[(0x8bf2,'FD','P'),(0x8c0a,'O','R'),(0x8c14,'O','R'),(0x8c1e,'O','R'),(0x8c28,'O','R'),(0x8c32,'O','X'),(0x8c3c,'O','X'),(0x8c46,'FE','H'),(0x8c4b,'FF','T')],
  0xD:[(0x8ca3,'FD','P'),(0x8cb4,'O','R'),(0x8cbe,'O','X'),(0x8cc8,'FD','H'),(0x8ccf,'O','X'),(0x8cd9,'FF','T')],
  0xE:[(0x8cfc,'FD','P'),(0x8d03,'O','X'),(0x8d0d,'O','X'),(0x8d17,'FE','E'),(0x8d1c,'FF','T')],
  0xF:[(0x8d20,'FD','P'),(0x8d31,'FB','C'),(0x8d36,'O','X'),(0x8d40,'O','X'),(0x8d4a,'FE','H'),(0x8d4f,'FD','H'),(0x8d56,'FF','T')],
  0x10:[(0x8d6b,'FD','P'),(0x8d7c,'O','R'),(0x8d86,'O','R'),(0x8d90,'O','X'),(0x8d9a,'FB','C'),(0x8da6,'FD','H'),(0x8dad,'FF','T')],
  0x11:[(0x8dd1,'FD','P'),(0x8de2,'O','R'),(0x8dec,'FD','H'),(0x8df3,'FB','C'),(0x8df8,'FB','C'),(0x8dfd,'FB','C'),(0x8e02,'O','X'),(0x8e0c,'FF','T')],
  0x20:[(0x923c,'FD','P'),(0x9243,'O','X'),(0x924d,'O','X'),(0x9257,'O','X'),(0x9261,'FE','E'),(0x9266,'FF','T')],
  0x21:[(0x926a,'FD','P'),(0x9271,'O','H'),(0x927b,'O','X'),(0x9285,'FE','X'),(0x928f,'O','V'),(0x929e,'FE','X'),(0x92a3,'O','X'),(0x92ad,'FE','H'),(0x92b2,'FF','T')],
}

def source_roster(rom):
    result=[]
    for map_id,records in RECORDS.items():
        require(u(rom,0x28000+map_id*2)==0,'changed scene-table fallback')
        pointer=u(rom,0x38000+map_id*2)
        rows=[]
        for offset,kind,role in records:
            at=0x30000+offset;length={'O':10,'FD':7,'FE':5,'FB':5,'FF':1}[kind]
            b=rom[at:at+length];require(len(b)==length,'short source record')
            require(b[0]<0xfa if kind=='O' else b[0]==int(kind,16),'changed record kind')
            row={'source':at|0x800000,'kind':kind,'disposition':role,'source_sha256':sha(b)}
            if kind=='FF':
                row.update(entry=0x858008,text_source=(at+1)|0x800000)
            else:
                header_at=4 if kind in ('O','FD') else 2
                header=int.from_bytes(b[header_at:header_at+3],'little')
                raw=header&0x3fffff;prefix=3 if kind=='FB' else 5
                require(header>=0x800000 and len(rom)>=raw+prefix,'invalid source actor header')
                row.update(header=header,entry=header+prefix,header_sha256=sha(rom[raw:raw+prefix]))
                if kind!='FB':row.update(initial_selector=rom[raw],initial_flags=[u(rom,raw+1),u(rom,raw+3)])
                if kind in ('O','FD'):row['position']=[b[1]*16+8,b[2]*16];row['parameter']=b[3]
                elif kind=='FE':row['position']=[8,0];row['parameter']=b[1]
                else:row['parameter']=b[1]
                if kind=='O':
                    row['descriptor']=int.from_bytes(b[7:10],'little')
                    if row['descriptor']:
                        d=row['descriptor']&0x3fffff
                        row['descriptor_sha256']=sha(rom[d:d+13])
                        row['composition_packet']=int.from_bytes(rom[d:d+3],'little')
                elif kind!='FB':row['implicit_animation_base']=0x9ad000
            rows.append(row)
        result.append({'map':map_id,'list':0x830000+pointer,'records':rows})
    return result
