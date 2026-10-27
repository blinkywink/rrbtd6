"""Bind original social-menu controls without online/account mutations."""
import json,sys,xml.etree.ElementTree as ET
from pathlib import Path
root=Path(sys.argv[1]);out={}
loc={t.attrib['id']:''.join(t.itertext()).replace('\\n','\n') for t in ET.parse(root/'text/English.xml').iter('T') if 'id' in t.attrib}
for file,kind in [('LinkFriendsUI','LinkFriendsScreen'),('TeamsBrowserUI','TeamsBrowserScreen'),('PerksUI','SocialSeasonsPerksScreenUI')]:
    scene=json.loads((root/'ui/prefabs'/f'{file}.json').read_text(encoding='utf8'));nodes=[]
    def walk(n,path=''):
        path=path+'/'+n['name'] if path else n['name'];nodes.append((path.removeprefix(file+'/'),n))
        for c in n['children']:walk(c,path)
    walk(scene)
    refs={str(c['id']):p for p,n in nodes for c in n['comps'] if 'id' in c};refs.update({str(n['id']):p for p,n in nodes})
    script=next(c for _,n in nodes for c in n['comps'] if c['type']==kind)
    def ref(key):return refs[script[key].removeprefix('->')]
    config={'initial':[],'tabs':[],'dialogs':[],'inputs':[],'text':[]}
    def active(path,on):config['initial'].append({'path':path,'on':on})
    def field(key,on):active(ref(key),on)
    def tabs(pairs):
        for button,panel in pairs:config['tabs'].append({'button':ref(button),'panel':ref(panel),'highlight':ref(button)+'/BgSelected'})
        for i,tab in enumerate(config['tabs']):active(tab['panel'],i==0);active(tab['highlight'],i==0)
    if file=='LinkFriendsUI':
        active('Canvas/LinkFriends/Container/TopBar/FriendCodePanal/CopyBtn/IosShareImg',False)
        active('Canvas/LinkFriends/Container/TopBar/FriendCodePanal/CopyBtn/AndroidShareImg',False)
        tabs([('friendsToggle','friendsTab'),('requestsToggle','requestsTab')])
        for tab,key in zip(config['tabs'],('FriendScreenFriendText','FriendScreenRequestText')):
            tab['text']=[{'path':ref('foundNothingText'),'text':loc[key]}]
        for key in ('searchingImage','friendCodePanel','friendRequestPip','friendCodeLoadingImg','copyCodeTick','friendCodeSuccessTick','discordRevokeContainer','discordLoadingImg','discordRevokeLoadingImg','discordPopupLoadingImg','discordLoadingInfoPopup'):field(key,False)
        field('foundNothingPanel',True)
        panel='Canvas/LinkFriends/AddFriendDiscordPopup'
        config['dialogs'].append({'panel':panel,'open':[ref('addFriendButton')],'close':[ref('closeAddFriendsButton')],
            'controller':'PopupWithBgAnim','open_clip':'PopupWithBgOpen','close_clip':'PopupWithBgClose'})
    elif file=='TeamsBrowserUI':
        tabs([('teamsAllTgl','openTeamsTab'),('teamsFriendsTgl','friendsTeamsTab'),('teamsRequestsTgl','requestsTeamsTab')])
        for key in ('screenClickBlockPanel','searchingPanel','paginationPanel','requestsPip'):field(key,False)
        field('nothingFoundPanel',True)
        config['text'].append({'path':ref('introPanelText'),'text':loc['TeamNoInfo']})
        config['dialogs'].append({'panel':ref('introPanel'),'open':[ref('introPanelButton')],
            'close':[ref('introBackgroundPanelCloseButton'),ref('introOkClosePanelButton')],
            'controller':'OdysseyIntroPanelAnim','open_clip':'OdysseyIntroPanelOpen','close_clip':'OdysseyIntroPanelClose'})
        # Keep native advanced-search layout available; service filter submission remains unbound.
        advanced=next(c for _,n in nodes for c in n['comps'] if c['type']=='TeamBrowserAdvancedPanel')
        config['dialogs'].append({'panel':ref('advSearchPanel'),'open':[ref('advSearchOptionsButton')],
            'close':[refs[advanced[k].removeprefix('->')] for k in ('applyButton','backgroundButton')],
            'controller':'TeamBrowserAdvSearch','open_clip':'TeamBrowserAdvSearch','close_clip':None})
    else:
        popup='Canvas/PerksScreen/PerksFirstTimePopup/Panel/Info/Viewport/Content'
        config['text'].append({'path':popup+'/Introduction/Body','text':loc['PerksInfoSubtitle']})
        for section,index in [('SectionOne',1),('SectionTwo',2),('SectionThree',3),('SectionFour',4)]:
            config['text'].append({'path':popup+'/'+section+'/Body','text':loc['PerksInfo'+str(index)]})
        for key in ('timerContainer','perkCoopContainer','noCoopPerksEnabled','divider','perkLockedContainer','perkPanel'):field(key,False)
        field('noPerksEarnedObj',True)
        popup=next(c for _,n in nodes for c in n['comps'] if c['type']=='SocialSeasonsPerksScreenFirstTimePopup')
        config['dialogs'].append({'panel':ref('firstTimePopup'),'open':[ref('infoButton')],
            'close':[refs[popup['okButton'].removeprefix('->')]],'controller':'PopupWithBgAnim','open_clip':'PopupWithBgOpen','close_clip':'PopupWithBgClose'})
    for key in ('addFriendInput','searchFieldInput'):
        if key not in script:continue
        path=ref(key)
        entry=next(n for p,n in nodes if p==path)
        component=next(c for c in entry['comps'] if c['type']=='NK_TextMeshProInputField')
        text=next((p for p,n in nodes if p.startswith(path+'/') and n['name'] in ('Text','SearchText') and any(c['type']=='TextMeshProUGUI' for c in n['comps'])),None)
        if text:
            placeholder=next((p for p,n in nodes if p.startswith(path+'/') and 'placeholder' in n['name'].lower()),None)
            config['inputs'].append({'path':path,'text_path':text,'placeholder':placeholder,'limit':component.get('m_CharacterLimit',0)})
            if file=='TeamsBrowserUI' and placeholder:config['text'].append({'path':placeholder,'text':loc['TeamBrowserSearchField']})
    paths={p for p,n in nodes}
    for dialog in config['dialogs']:
        panel=dialog['panel']
        surface=next((panel+'/'+n for n in ('Panel','InfoPanel') if panel+'/'+n in paths),panel)
        dialog['surface']=surface
        dialog['outside']=[p for p in dialog['close'] if p.endswith(('/Close','/ClickableBackground'))]
        dialog['close']=[p for p in dialog['close'] if p not in dialog['outside']]
        done=panel+'/Panel/Layout/Buttons/CanelButton'
        if done in paths:dialog['close'].append(done)
        active(panel,False)
    for p,n in nodes:
        if n['name'] in ('JewelBar','DebugFriendsUI','SyncingStates') or any(c['type'] in ('LinkFriendPanel','DiscordFriendPanel','LiNKGuildPanel','SocialSeasonsPerkUI') for c in n['comps']):active(p,False)
    if file=='PerksUI':active(ref('perkTemplate'),False)
    out[file]=config
(root/'ui/runtime/social_controls.json').write_text(json.dumps(out,indent=1),encoding='utf8')
print('Source controls:',[(k,len(v['tabs']),len(v['dialogs'])) for k,v in out.items()])

