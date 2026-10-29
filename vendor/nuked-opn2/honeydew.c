/* Honeydew host adapter, GPL-3.0-or-later. Nuked-OPN2 remains LGPL-2.1-or-later. */
#include "ym3438.h"
#include <string.h>
typedef struct {
    ym3438_t chip;
    Bit8u desired[512];
    Bit32u dirty[16], valid[16];
    Bit16u count, current, return_low;
    Bit8u wait, data_pending, key_pending, retrigger, current_data, low_data;
    Bit8u key[6], locked_pan;
    Bit16u pan[6][2];
} hd_ym;
typedef char state_size_guard[(sizeof(hd_ym) <= 4096) ? 1 : -1];
void hd_ym_init_type(void) { OPN2_SetChipType(ym3438_mode_ym2612); }
void hd_ym_reset(hd_ym *s) { memset(s,0,sizeof(*s)); OPN2_Reset(&s->chip); s->return_low=0xffff; for(unsigned i=0;i<6;i++){s->pan[i][0]=s->pan[i][1]=256;} }
static void clear_dirty(hd_ym *s, unsigned r) {
    Bit32u bit=1u<<(r&31);
    if(s->dirty[r>>5]&bit) { s->dirty[r>>5]&=~bit; --s->count; }
}
void hd_ym_write(hd_ym *s, unsigned r, unsigned value) {
    r&=511; value&=255;
    if(r==0x28) {
        unsigned ch=value&7; if(ch==3||ch==7)return; if(ch>=4)--ch;
        s->key[ch]=(Bit8u)value; s->key_pending|=1u<<ch;
        if(value&0xf0)s->retrigger|=1u<<ch; else s->retrigger&=~(1u<<ch);
        return;
    }
    Bit32u bit=1u<<(r&31); unsigned word=r>>5;
    if((s->valid[word]&bit) && s->desired[r]==value)return;
    s->valid[word]|=bit; s->desired[r]=(Bit8u)value;
    if(!(s->dirty[word]&bit)){s->dirty[word]|=bit; ++s->count;}
}
static int next_write(hd_ym *s) {
    unsigned r=0; Bit8u data=0;
    if(s->return_low!=0xffff) { r=s->return_low; s->return_low=0xffff; data=s->low_data; }
    else if(s->count) {
        while(!s->dirty[r>>5])r+=32;
        while(!(s->dirty[r>>5]&(1u<<(r&31))))++r;
        /* FNUM high is a shared latch: always write it immediately before its channel's low byte. */
        if((r&0xfc)==0xa0) { s->return_low=(Bit16u)r; s->low_data=s->desired[r]; clear_dirty(s,r); r+=4; }
        data=s->desired[r]; clear_dirty(s,r);
    } else if(s->key_pending) {
        unsigned ch=0; while(!(s->key_pending&(1u<<ch)))++ch;
        r=0x28;
        if(s->retrigger&(1u<<ch)) { s->retrigger&=~(1u<<ch); data=(Bit8u)(ch+(ch>=3)); }
        else {s->key_pending&=~(1u<<ch); data=s->key[ch];}
    } else return 0;
    s->current=(Bit16u)r; s->current_data=data; OPN2_Write(&s->chip,(r>>8)*2,(Bit8u)r);
    s->wait=1; s->data_pending=1; return 1;
}
void hd_ym_sample(hd_ym *s, int *out) {
    short sample[2]; out[0]=out[1]=0;
    for(unsigned i=0;i<24;i++) {
        if(s->wait)--s->wait;
        else if(s->data_pending) {
            OPN2_Write(&s->chip,(s->current>>8)*2+1,s->current_data);
            s->data_pending=0; s->wait=32;
        } else next_write(s);
        if((s->chip.cycles&3)==0)s->locked_pan=(Bit8u)(s->chip.channel+(s->chip.cycles<12));
        unsigned channel=s->locked_pan;
        OPN2_Clock(&s->chip,sample);
        out[0]+=(sample[0]-3)*s->pan[channel][0]/256;
        out[1]+=(sample[1]-3)*s->pan[channel][1]/256;
    }
}
void hd_ym_levels(const hd_ym *s, unsigned short *out) {
    for(unsigned i=0;i<24;i++)out[i]=s->chip.eg_out[i];
}

void hd_ym_pan(hd_ym *s,unsigned ch,unsigned left,unsigned right){if(ch<6){s->pan[ch][0]=(Bit16u)left;s->pan[ch][1]=(Bit16u)right;}}
unsigned hd_ym_lfo(const hd_ym *s){return s->chip.lfo_cnt;}
