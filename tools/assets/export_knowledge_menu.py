"""Extract read-only Monkey Knowledge card bindings from the original tree and models."""
import json
from pathlib import Path
import sys
import xml.etree.ElementTree as ET

root=Path(sys.argv[1])
scene=json.loads((root/'ui/prefabs/KnowledgeSkillTreeUI.json').read_text())
loc={t.attrib['id']:''.join(t.itertext()) for t in ET.parse(root/'text/English.xml').iter('T') if 'id' in t.attrib}
models={}
for line in (root/'decoded/named/objects.jsonl').open(encoding='utf8'):
    item=json.loads(line)
    if item['type']=='KnowledgeModel':models[item['f']['_name']]=item['f']
# KnowledgeModel.prerequisiteIds are Newtonsoft $refs into the decoded containers.
containers=json.loads((root/'decoded/named/containers.json').read_text(encoding='utf8'))
nodes=[];refs={}
def walk(node,path):
    nodes.append((node,path))
    for key in ('id','transform_id'):
        if key in node:refs[node[key]]=(node,path)
    for component in node.get('comps',[]):
        if 'id' in component:refs[component['id']]=(node,path)
    for child in node.get('children',[]):walk(child,path+'/'+child['name'] if path else child['name'])
for child in scene['children']:walk(child,child['name'])
entries=[];unresolved=[]
for node,path in nodes:
    script=next((c for c in node.get('comps',[]) if c['type']=='KnowledgeSkillBtn'),None)
    if not script:continue
    name=path.rsplit('/',2)[-2]
    if name not in models:unresolved.append(name);continue
    model=models[name]
    icon_node,_=refs[int(script['icon'][2:])]
    image=next(c['sprite'] for c in icon_node['comps'] if c['type']=='Image')
    effect=refs[int(script['selectedEffect'][2:])][1]
    category=model['category'];required=model['investmentRequired']
    category_name=['Primary','Military','Magic','Support','Heroes','Powers'][category]
    requirement=loc['Knowledge Requirement'].replace('{0}',str(required)).replace('{1}',loc.get(category_name,category_name)) if required else ''
    entries.append({'id':name,'path':path,'icon':image,'effect':effect,'category':category,
                    'title':loc.get(name,name),'description':loc.get(name+'Description',''),
                    'cost':model['monkeyMoneyCost'],'requirement':requirement,'raw':model,
                    # KnowledgeSkillTree.UpdateSelectedSkill text: "Needs" + missing prerequisite
                    # names, or Format("Knowledge Requirement", required - learnt, category name).
                    'prerequisites':containers.get(str(model['prerequisiteIds'].get('$ref')),[]) if isinstance(model.get('prerequisiteIds'),dict) else model.get('prerequisiteIds') or [],
                    'required':required,'categoryName':loc.get(category_name,category_name),
                    'needsText':loc.get('Needs','Needs'),'requirementText':loc['Knowledge Requirement'],
                    # KnowledgeSkillBtn.Construct: the node's original Icon is destroyed and the
                    # Images under its Arrows child are tinted by SetState / Update.
                    'nodeIcon':path.rsplit('/',1)[0]+'/Icon',
                    'arrows':[path.rsplit('/',1)[0]+'/Arrows/'+a['name'] for c in refs[node['id']][0:1]
                              for parent in [next((n for n,q in nodes if q==path.rsplit('/',1)[0]),None)] if parent
                              for arrows in parent['children'] if arrows['name']=='Arrows' for a in arrows['children']],
                    'lockedArrowColor':script.get('lockedArrowColor'),'purchasedArrowColor':script.get('purchasedArrowColor'),
                    'arrowFade':script.get('animateArrowsLength',0)})
dest=root/'ui/runtime/knowledge_items.json'
dest.write_text(json.dumps(entries,indent=1),encoding='utf8')
print(len(entries),'knowledge menu entries; unresolved tree nodes:',unresolved)
