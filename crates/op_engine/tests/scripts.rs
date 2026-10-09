use op_engine::Engine;
use op_net::{LoadError, NetworkContext};

#[test]
fn m428_stop_immediate_event_reuse_and_native_repaint() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<body><main id='outer'><button id='inner'>BUTTON</button></main>",
            "<p id='result'>WAIT</p><script>",
            "var outer=document.getElementById('outer');",
            "var inner=document.getElementById('inner');",
            "var order='';",
            "function halt(e){order=order+'A';e.stopImmediatePropagation();}",
            "inner.addEventListener('signal',halt);",
            "inner.addEventListener('signal',function(){order=order+'B';});",
            "outer.addEventListener('signal',function(){order=order+'P';});",
            "var event=new Event('signal',{bubbles:true});",
            "var first=inner.dispatchEvent(event);",
            "inner.removeEventListener('signal',halt);",
            "var second=inner.dispatchEvent(event);",
            "document.getElementById('result').textContent=",
            " first&&second&&order==='ABP'&&event.eventPhase===0&&",
            " event.currentTarget===null?",
            " 'M428-STOPIMMEDIATE-PASS':'M428-STOPIMMEDIATE-FAIL';",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|command| matches!(
        command,
        op_paint::PaintCommand::Text { text, .. }
            if text.contains("M428-STOPIMMEDIATE-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m427_matches_and_closest_use_compound_selector_chains() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<body><main id='outer' class='page'>",
            "<section id='section' class='box'><p id='target' class='item active'>OK</p>",
            "</section></main><p id='result'>WAIT</p><script>",
            "var target=document.getElementById('target');",
            "var section=document.getElementById('section');",
            "var outer=document.getElementById('outer');",
            "var matched=target.matches('p.item.active') && ",
            " target.matches('main.page > section.box > p#target') && ",
            " !target.matches('.missing') && !target.matches('section');",
            "var parents=target.closest('section.box')===section && ",
            " target.closest('main.page')===outer && ",
            " target.closest('p.item')===target && ",
            " target.closest('.absent')===null && ",
            " target.closest('#outer, section.box')===section;",
            "document.getElementById('result').textContent=",
            " matched&&parents?'M427-MATCHES-PASS':'M427-MATCHES-FAIL';",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M427-MATCHES-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m427_original_legacy_event_and_returnvalue_sync() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<body><button id='target'>BTN</button><p id='result'>WAIT</p><script>",
            "var node=document.getElementById('target');",
            "var e=document.createEvent('Event');",
            "var uninitialized=e.type==='' && e.target===null;",
            "e.initEvent('update',false,true);",
            "node.addEventListener('update',function(event){event.returnValue=false;});",
            "var accepted=node.dispatchEvent(e);",
            "document.getElementById('result').textContent=",
            " uninitialized && !accepted && e.defaultPrevented && !e.returnValue?",
            " 'M427-LEGACY-PASS':'M427-LEGACY-FAIL';",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M427-LEGACY-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m427_same_event_cannot_dispatch_recursively() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<body><button id='target'>BTN</button><p id='result'>WAIT</p><script>",
            "var node=document.getElementById('target');",
            "var event=new Event('repeat');",
            "var rejected=false;",
            "node.addEventListener('repeat',function(){",
            " try{node.dispatchEvent(event)}catch(e){rejected=true}",
            "});",
            "var accepted=node.dispatchEvent(event);",
            "document.getElementById('result').textContent=",
            " accepted&&rejected?'M427-REENTRY-PASS':'M427-REENTRY-FAIL';",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M427-REENTRY-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m427_compound_child_descendant_and_comma_selectors() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<body><main id='root' class='screen'><section class='container'>",
            "<p id='first' class='item active'>ONE</p>",
            "<aside><p id='second' class='item'>TWO</p></aside>",
            "</section></main><p class='item' id='outside'>OUTSIDE</p>",
            "<p id='result'>WAIT</p><script>",
            "var root=document.getElementById('root');",
            "var first=document.getElementById('first');",
            "var second=document.getElementById('second');",
            "var direct=root.querySelector('section > p.item.active')===first;",
            "var descendant=root.querySelector('section p.item')===first && ",
            " root.querySelector('main.screen p.item')===first;",
            "var grouped=root.querySelectorAll('p#first.active, aside > p#second');",
            "var groups=grouped.length===2 && grouped[0]===first && grouped[1]===second;",
            "var noDuplicate=root.querySelectorAll('p.item, .active').length===2;",
            "var limited=root.querySelector('p#outside')===null && ",
            " root.querySelector('section > aside > p.item')===second;",
            "var rejected=false;",
            "try{root.querySelector('p:hover')}catch(e){rejected=true}",
            "document.getElementById('result').textContent=",
            " direct&&descendant&&groups&&noDuplicate&&limited&&rejected?",
            " 'M427-SELECTORS-PASS':'M427-SELECTORS-FAIL';",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M427-SELECTORS-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m427_custom_event_capture_target_bubble_cancelation() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<body><main id='outer'><button id='inner'>BTN</button></main>",
            "<p id='result'>WAIT</p><script>",
            "var parent=document.getElementById('outer');",
            "var button=document.getElementById('inner');",
            "var order='';",
            "parent.addEventListener('change',function(e){",
            " order=order+'C'+e.eventPhase;",
            "},true);",
            "button.addEventListener('change',function(e){",
            " order=order+'T'+e.eventPhase;",
            " if(e.target===button && e.currentTarget===button)e.preventDefault();",
            "});",
            "parent.addEventListener('change',function(e){",
            " order=order+'B'+e.eventPhase;",
            "});",
            "var evt=new Event('change',{bubbles:true,cancelable:true});",
            "var uncanceled=button.dispatchEvent(evt);",
            "var done=!uncanceled && evt.defaultPrevented && ",
            " evt.eventPhase===0 && evt.currentTarget===null && ",
            " evt.target===button && order==='C1T2B3';",
            "document.getElementById('result').textContent=",
            " done?'M427-EVENT-PASS':'M427-EVENT-FAIL';",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M427-EVENT-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m427_event_nonbubbling_and_stop_propagation() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<body><main id='outer'><button id='inner'>BTN</button></main>",
            "<p id='result'>WAIT</p><script>",
            "var parent=document.getElementById('outer');",
            "var child=document.getElementById('inner');",
            "var order='';",
            "parent.addEventListener('select',function(e){order=order+'CAP';},true);",
            "parent.addEventListener('select',function(e){order=order+'BUB';});",
            "child.addEventListener('select',function(e){",
            " order=order+'TARGET';e.preventDefault();",
            "});",
            "var evt=new Event('select',{bubbles:false,cancelable:false});",
            "var accepted=child.dispatchEvent(evt);",
            "var nonbubble=accepted && !evt.defaultPrevented && order==='CAPTARGET';",
            "child.addEventListener('hold',function(e){e.stopPropagation();order=order+'STOP';});",
            "parent.addEventListener('hold',function(e){order=order+'WRONG';});",
            "child.dispatchEvent(new Event('hold',{bubbles:true}));",
            "document.getElementById('result').textContent=",
            " nonbubble && order==='CAPTARGETSTOP'?",
            " 'M427-STOP-PASS':'M427-STOP-FAIL';",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M427-STOP-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m427_event_target_timer_and_object_reuse() {
    let mut engine = Engine::new();
    let initial = engine.set_html_page(
        concat!(
            "<body><section id='region'></section><p id='result'>WAIT</p>",
            "<script>",
            "var root=document.getElementById('region');",
            "var child=document.createElement('button');",
            "root.appendChild(child);",
            "var seen=0;",
            "child.addEventListener('save',function(e){seen=seen+1;});",
            "var evt=new Event('save',{bubbles:true});",
            "setTimeout(function(){",
            " var first=child.dispatchEvent(evt);",
            " var second=child.dispatchEvent(evt);",
            " document.getElementById('result').textContent=",
            " first&&second&&seen===2 && evt.target===child?",
            " 'M427-TIMER-PASS':'M427-TIMER-FAIL';",
            "},0);",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(!initial.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M427-TIMER-PASS")
    )));
    let after = engine
        .tick_timers(800, 600)
        .expect("event dispatched from timer");
    assert!(after.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M427-TIMER-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m426_programmatic_click_bubbles_and_repaints_native_dom() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<body><main id='outer'><button id='action'>Action</button></main>",
            "<p id='result'>WAIT</p><script>",
            "var button=document.getElementById('action');",
            "var outer=document.getElementById('outer');",
            "var result=document.getElementById('result');",
            "var count=0;",
            "button.addEventListener('click',function(event){",
            " count=count+1;",
            " if(event.type==='click' && event.target===button && event.eventPhase===2)",
            "   result.textContent='TARGET';",
            "});",
            "outer.addEventListener('click',function(event){",
            " if(event.eventPhase===3 && event.target===button && count===1)",
            "   result.textContent='M426-CLICK-PASS';",
            "});",
            "var returned=button.click();",
            "if(returned!==undefined) result.textContent='M426-CLICK-WRONG-RETURN';",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M426-CLICK-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m426_programmatic_click_from_timer_uses_existing_handlers() {
    let mut engine = Engine::new();
    let initial = engine.set_html_page(
        concat!(
            "<body><button id='action'>Action</button><p id='result'>WAIT</p>",
            "<script>",
            "var b=document.getElementById('action');",
            "b.onclick=function(event){",
            "document.getElementById('result').textContent=",
            " event.target===b?'M426-TIMER-CLICK-PASS':'M426-TIMER-CLICK-FAIL';",
            "};",
            "setTimeout(function(){b.click();},0);",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(!initial.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M426-TIMER-CLICK-PASS")
    )));
    let after = engine
        .tick_timers(800, 600)
        .expect("timer dispatches click");
    assert!(after.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M426-TIMER-CLICK-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m426_query_selector_static_results_and_scoped_live_lookup() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<body><main id='host'><p id='one' class='item'>ONE</p></main>",
            "<p id='outside' class='item'>OUTSIDE</p><p id='result'>WAIT</p>",
            "<script>",
            "var host=document.querySelector('#host');",
            "var one=document.getElementById('one');",
            "var first=host.querySelector('p')===one && host.querySelector('.item')===one && ",
            " document.querySelector('#outside')===document.getElementById('outside');",
            "var old=host.querySelectorAll('.item');",
            "var created=document.createElement('p');",
            "created.className='item';created.id='added';created.textContent='ADDED';",
            "host.appendChild(created);",
            "var newest=host.querySelectorAll('.item');",
            "var snapshot=old.length===1 && old.item(0)===one && old[1]===undefined && ",
            " newest.length===2 && newest[0]===one && newest.item(1)===created;",
            "var scoped=host.querySelector('#outside')===null && ",
            " document.querySelector('main')===host && ",
            " host.querySelector('*')===one && host.querySelector('.absent')===null;",
            "created.remove();",
            "var retained=newest.length===2 && newest[1]===created && ",
            " host.querySelectorAll('.item').length===1 && ",
            " host.getElementsByTagName('p').length===1;",
            "var supported=host.querySelector('main p')===one;",
            "document.getElementById('result').textContent=",
            " first&&snapshot&&scoped&&retained&&supported?",
            " 'M426-QUERY-PASS':'M426-QUERY-FAIL';",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M426-QUERY-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m426_query_snapshot_survives_native_binding_and_timer() {
    let mut engine = Engine::new();
    let initial = engine.set_html_page(
        concat!(
            "<body><script>",
            "var host=document.createElement('section');",
            "var child=document.createElement('p');",
            "child.className='notice';child.textContent='ORIGINAL';",
            "host.appendChild(child);",
            "var snapshot=host.querySelectorAll('.notice');",
            "document.body.appendChild(host);",
            "setTimeout(function(){",
            "var preserved=snapshot.length===1 && snapshot[0]===child && ",
            " host.querySelector('.notice')===child;",
            "var second=document.createElement('p');",
            "second.className='notice';host.appendChild(second);",
            "var staticOld=snapshot.length===1 && ",
            " host.querySelectorAll('.notice').length===2;",
            "child.textContent=preserved&&staticOld?'M426-TIMER-PASS':'M426-TIMER-FAIL';",
            "},0);",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(!initial.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M426-TIMER-PASS")
    )));
    let changed = engine
        .tick_timers(800, 600)
        .expect("timer changes query source");
    assert!(changed.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M426-TIMER-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m426_in_operator_uses_native_dom_properties_and_validates_rhs() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<body><main id='host'><b id='child'>B</b></main><p id='result'>WAIT</p>",
            "<script>",
            "var host=document.getElementById('host');",
            "var obj={a:undefined,b:3};",
            "var native='childElementCount' in host && 'children' in host && ",
            " 'nodeName' in host && !('notARealProperty' in host);",
            "var ordinary='a' in obj && 'b' in obj && !('missing' in obj);",
            "var threw=false;",
            "try{var nonsense='a' in null;}catch(error){threw=true}",
            "document.getElementById('result').textContent=",
            " native&&ordinary&&threw?'M426-IN-PASS':'M426-IN-FAIL';",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M426-IN-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m425_live_tag_collections_and_attributes_repaint_after_mutations() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<body><main id='host'><b id='initial'>BASE</b></main><p id='result'>START</p>",
            "<script>",
            "var all=document.getElementsByTagName('b');",
            "var host=document.getElementById('host');",
            "var scoped=host.getElementsByTagName('b');",
            "var initial=all.length===1 && all[0]===scoped.item(0) && ",
            "  scoped.length===1 && all.item(99)===null;",
            "var extra=document.createElement('b');",
            "var noAttrs=!extra.hasAttributes() && !extra.hasAttribute('title');",
            "extra.setAttribute('title','inserted');",
            "var attrs=extra.hasAttributes() && extra.hasAttribute('TITLE') && ",
            " extra.getAttribute('title')==='inserted';",
            "host.appendChild(extra);",
            "var inserted=all.length===2 && scoped.length===2 && ",
            " all[1]===extra && scoped.item(1)===extra;",
            "extra.removeAttribute('title');",
            "var gone=!extra.hasAttribute('title') && !extra.hasAttributes();",
            "extra.remove();",
            "var removed=all.length===1 && scoped.length===1 && ",
            " scoped.item(0)===document.getElementById('initial');",
            "document.getElementById('result').textContent=",
            " initial&&noAttrs&&attrs&&inserted&&gone&&removed?",
            " 'M425-TAG-LIVE-PASS':'M425-TAG-LIVE-FAIL';",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M425-TAG-LIVE-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m425_tag_collection_persists_across_delayed_dom_mutations() {
    let mut engine = Engine::new();
    let before = engine.set_html_page(
        concat!(
            "<body><main id='host'></main><p id='result'>WAITING</p><script>",
            "var host=document.getElementById('host');",
            "var collection=document.getElementsByTagName('mark');",
            "setTimeout(function(){",
            "var added=document.createElement('mark');",
            "host.appendChild(added);",
            "var now=collection.length===1 && collection.item(0)===added;",
            "added.setAttribute('id','late');",
            "var valid=now && host.getElementsByTagName('mark')[0]===added;",
            "document.getElementById('result').textContent=",
            " valid?'M425-TIMER-PASS':'M425-TIMER-FAIL';",
            "},0);",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(!before.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M425-TIMER-PASS")
    )));
    let after = engine
        .tick_timers(800, 600)
        .expect("timer DOM mutation should repaint");
    assert!(after.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M425-TIMER-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m424_live_element_children_collection_ignores_text_nodes() {
    let mut engine = Engine::new();
    let page=engine.set_html_page(concat!(
        "<main id='host'>plain <b id='first'>FIRST</b> mid <i name='named'>SECOND</i> tail</main>",
        "<script>",
        "var host=document.getElementById('host');",
        "var children=host.children;",
        "var first=document.getElementById('first');",
        "var initially=children===host.children && children.length===2 && ",
        " children[0]===first && children.item(0)===first && ",
        " children.namedItem('first')===first && ",
        " children.namedItem('named')===children[1] && ",
        " children.namedItem('absent')===null && ",
        " children.item(100)===null && host.childElementCount===2;",
        "var other=document.createElement('em');",
        "other.id='third';other.textContent='THIRD';",
        "host.appendChild(other);",
        "var inserted=children.length===3 && children[2]===other && ",
        " children.namedItem('third')===other && host.lastElementChild===other;",
        "first.remove();",
        "var removed=children.length===2 && children[0]!==first && ",
        " children.namedItem('first')===null && ",
        " host.firstElementChild===children[0] && host.childElementCount===2;",
        "other.textContent=(initially && inserted && removed)?",
        " 'M424-CHILDREN-PASS':'M424-CHILDREN-FAIL';",
        "</script>"
    ),800,600);
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M424-CHILDREN-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m424_children_live_across_timer_and_native_binding() {
    let mut engine = Engine::new();
    let initial = engine.set_html_page(
        concat!(
            "<body><script>",
            "var parent=document.createElement('section');",
            "var children=parent.children;",
            "document.body.appendChild(parent);",
            "setTimeout(function(){",
            " var child=document.createElement('b');",
            " child.textContent='M424-TIMER-FAIL';",
            " parent.appendChild(child);",
            " if(children===parent.children && children.length===1 && ",
            "   children.item(0)===child && child.parentNode===parent) ",
            "   child.textContent='M424-TIMER-PASS';",
            "},0);",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(!initial.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M424-TIMER-PASS")
    )));
    let after = engine.tick_timers(800, 600).expect("timer creates element");
    assert!(after.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M424-TIMER-PASS")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m423_classlist_variadic_mutations_are_atomic() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<main id='host'><p id='target' class='original'>CLASS-423</p></main>",
            "<script>",
            "var el=document.getElementById('target');",
            "var tokens=el.classList;",
            "tokens.add('new','another','new');",
            "var added=tokens.length===3 && tokens.contains('new') && tokens.contains('another');",
            "var threw=false;",
            "try { tokens.add('valid','invalid token'); } catch(error) { threw=true; }",
            "var atomic=tokens.length===3 && !tokens.contains('valid');",
            "tokens.remove('original','another');",
            "var removed=tokens.length===1 && tokens[0]==='new' && el.className==='new';",
            "tokens.remove();tokens.add();",
            "el.textContent=(added&&threw&&atomic&&removed&&tokens.length===1)?",
            " 'VARIADIC-PASSED-423':'VARIADIC-FAILED-423';",
            "</script>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("VARIADIC-PASSED-423")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m423_css_scanner_preserves_semicolon_in_quotes_and_function() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<main id='host'><p id='target'>CSS-423</p></main>",
            "<script>",
            "var el=document.getElementById('target');",
            "el.style.cssText='background-image: url(\"data:image/svg+xml;a:b\"); display: block';",
            "var before=el.style.getPropertyValue('background-image')===",
            " 'url(\"data:image/svg+xml;a:b\")' && el.style.display==='block';",
            "el.style.setProperty('display','none');",
            "var after=el.style.getPropertyValue('background-image')===",
            " 'url(\"data:image/svg+xml;a:b\")' && el.style.display==='none';",
            "if(!before || !after) el.style.display='block';",
            "</script>"
        ),
        800,
        600,
    );
    assert!(!page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("CSS-423")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m423_live_siblings_node_identity_connectivity_and_contains() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<body><main id='host'><b id='a'>A</b><i id='b'>B</i><em id='c'>C</em></main>",
            "<script>",
            "var host=document.getElementById('host');",
            "var a=document.getElementById('a');",
            "var b=document.getElementById('b');",
            "var c=document.getElementById('c');",
            "var original=a.nodeName==='B' && a.tagName==='B' && ",
            " a.nodeType===1 && a.nodeValue===null && a.previousSibling===null && ",
            " a.nextSibling===b && b.previousSibling===a && b.nextSibling===c && ",
            " c.nextSibling===null && c.previousSibling===b && ",
            " a.ownerDocument===document && host.contains(a) && ",
            " host.contains(host) && !a.contains(host) && !a.contains(null) && ",
            " a.isConnected && host.isConnected;",
            "var saved=b;",
            "host.removeChild(b);",
            "var detached=!b.isConnected && b.previousSibling===null && b.nextSibling===null && ",
            " a.nextSibling===c && c.previousSibling===a && !host.contains(b);",
            "host.appendChild(b);",
            "var attached=b===saved && b.isConnected && c.nextSibling===b && ",
            " b.previousSibling===c && b.nextSibling===null;",
            "var text=document.createTextNode('TEXT');",
            "host.appendChild(text);",
            "var textProps=text.nodeName==='#text' && text.nodeType===3 && ",
            " text.length===4 && text.ownerDocument===document && ",
            " text.previousSibling===b && host.contains(text);",
            "var result=document.createElement('p');",
            "result.textContent=(original && detached && attached && textProps)?",
            " 'M423-SIBLINGS-OK':'M423-SIBLINGS-FAIL';",
            "host.appendChild(result);",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M423-SIBLINGS-OK")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m422_classlist_remove_and_camelcase_style_reflect_attributes() {
    let mut engine = Engine::new();
    let page=engine.set_html_page(concat!(
        "<main id='host'><p id='message' class='first second'>BEFORE-422-METHODS</p></main>",
        "<script>",
        "var p=document.getElementById('message');",
        "var tokens=p.classList;",
        "var initial=tokens.contains('first') && tokens.contains('second') && tokens.length===2;",
        "tokens.remove('first');",
        "tokens.add('third');",
        "var updated=p.getAttribute('class')==='second third' && ",
        " tokens.contains('third') && !tokens.contains('first');",
        "var error=false;",
        "try{tokens.add('invalid token')}catch(e){error=true}",
        "p.style.backgroundColor='red';",
        "var css=p.style.getPropertyValue('background-color')==='red' && ",
        " p.getAttribute('style')==='background-color: red';",
        "p.style.cssText='color: blue; display: block';",
        "var written=p.style.getPropertyValue('color')==='blue' && ",
        " p.style.display==='block';",
        "p.textContent=(initial&&updated&&error&&css&&written)?'METHODS-PASSED-422':'METHODS-FAILED-422';",
        "</script>"
    ),800,600);
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("METHODS-PASSED-422")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m422_created_node_saved_live_handles_survive_physical_binding() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<body><script>",
            "var p=document.createElement('section');",
            "p.id='created';",
            "var list=p.childNodes;",
            "var classes=p.classList;",
            "var css=p.style;",
            "classes.add('bound');",
            "css.setProperty('display','block');",
            "document.body.appendChild(p);",
            "setTimeout(function(){",
            " var same=p===document.getElementById('created') && ",
            " list===p.childNodes && classes===p.classList && css===p.style;",
            " var child=document.createElement('b');",
            " child.textContent=(same && classes.contains('bound') && ",
            " css.display==='block')?'BOUND-OK-422':'BOUND-BAD-422';",
            " p.appendChild(child);",
            " if(list.length!==1 || list[0]!==child || child.parentNode!==p)",
            "   child.textContent='BOUND-LIST-BAD-422';",
            "},0);",
            "</script></body>"
        ),
        800,
        600,
    );
    assert!(!page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("BOUND-OK-422")
    )));
    let updated = engine
        .tick_timers(800, 600)
        .expect("new node handles should survive commit");
    assert!(updated.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("BOUND-OK-422")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m422_style_object_methods_and_properties_repaint_after_timers() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<main id='host'><p id='message'>VISIBLE-M422-STYLE</p></main>",
            "<script>",
            "var node=document.getElementById('message');",
            "var style=node.style;",
            "var initial=style===node.style && style.getPropertyValue('display')==='';",
            "style.setProperty('display','none');",
            "var set=style.display==='none' && style.getPropertyValue('display')==='none';",
            "var removed=style.removeProperty('display')==='none';",
            "var cleared=style.display==='' && style.cssText==='';",
            "style.display=(initial && set && removed && cleared)?'none':'block';",
            "setTimeout(function(){",
            " if(style.display==='none') style.setProperty('display','block');",
            "},0);",
            "</script>"
        ),
        800,
        600,
    );
    assert!(!page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("VISIBLE-M422-STYLE")
    )));
    let updated = engine
        .tick_timers(800, 600)
        .expect("style timer causes repaint");
    assert!(updated.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("VISIBLE-M422-STYLE")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m422_child_nodes_live_collection_updates_across_timer() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<main id='host'><p id='one'>START-M422</p></main>",
            "<script>",
            "var host=document.getElementById('host');",
            "var nodes=host.childNodes;",
            "setTimeout(function(){",
            " var two=document.createElement('p');",
            " two.textContent=nodes.length===1?'TIMER-LIVE-M422':'TIMER-BROKEN-M422';",
            " host.appendChild(two);",
            " if(nodes.length===2 && nodes[1]===two && two.parentNode===host)",
            "   two.textContent='TIMER-LIVE-PASSED-M422';",
            "},0);",
            "</script>"
        ),
        800,
        600,
    );
    assert!(!page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("TIMER-LIVE-PASSED-M422")
    )));
    let updated = engine
        .tick_timers(800, 600)
        .expect("timer must modify live NodeList");
    assert!(updated.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("TIMER-LIVE-PASSED-M422")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m422_live_parent_node_and_child_nodes_track_mutations() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<main id='host'><p id='one'>OLD-422</p></main>",
            "<script>",
            "var host=document.getElementById('host');",
            "var first=document.getElementById('one');",
            "var nodes=host.childNodes;",
            "var initially=nodes.length===1 && nodes[0]===first && nodes.item(0)===first && ",
            " first.parentNode===host && host.firstChild===first && host.lastChild===first;",
            "var added=document.createElement('b');",
            "added.textContent='LIVE-422';",
            "host.appendChild(added);",
            "var afterAdd=nodes===host.childNodes && nodes.length===2 && nodes[1]===added && ",
            " added.parentNode===host && host.lastChild===added;",
            "host.removeChild(first);",
            "var afterRemove=nodes.length===1 && nodes.item(0)===added && ",
            " first.parentNode===null && host.firstChild===added && nodes.item(5)===null;",
            "if(initially && afterAdd && afterRemove) added.textContent='LIVE-PASSED-422';",
            "else added.textContent='LIVE-FAILED-422';",
            "</script>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("LIVE-PASSED-422")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m422_text_node_live_list_preserves_identity() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<main id='host'></main><script>",
            "var host=document.getElementById('host');",
            "var nodes=host.childNodes;",
            "var text=document.createTextNode('TEXT-422');",
            "host.appendChild(text);",
            "var correct=nodes.length===1 && nodes.item(0)===text && ",
            " text.parentNode===host && host.firstChild===text;",
            "text.data=correct?'TEXT-LIVE-422':'TEXT-BROKEN-422';",
            "</script>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("TEXT-LIVE-422")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m422_replace_child_preserves_old_identity_and_dom_order() {
    let mut engine = Engine::new();
    let page=engine.set_html_page(concat!(
        "<main id='host'><p id='old'>TO-REPLACE-422</p></main><script>",
        "var host=document.getElementById('host');",
        "var old=document.getElementById('old');",
        "var newNode=document.createElement('b');",
        "newNode.id='new';newNode.textContent='REPLACED-422';",
        "var before=host.childNodes.length;",
        "var removed=host.replaceChild(newNode,old);",
        "var valid=removed===old && old.parentNode===null && ",
        " newNode.parentNode===host && host.childNodes[0]===newNode && ",
        " document.getElementById('old')===null && ",
        " document.getElementById('new')===newNode && before===1 && host.childNodes.length===1;",
        "newNode.textContent=valid?'REPLACE-PASSED-422':'REPLACE-FAILED-422';",
        "</script>"
    ),800,600);
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("REPLACE-PASSED-422")
    )));
    assert!(!page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("TO-REPLACE-422")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m422_class_list_changes_real_selector_visibility() {
    let mut engine = Engine::new();
    let page=engine.set_html_page(concat!(
        "<style>.hidden { display:none }</style>",
        "<main id='host'><p id='message' class='old'>MESSAGE-422</p></main><script>",
        "var p=document.getElementById('message');",
        "var classes=p.classList;",
        "var same=classes===p.classList && classes.length===1 && classes[0]==='old';",
        "classes.add('hidden');",
        "var hidden=classes.contains('hidden') && p.className==='old hidden' && ",
        " classes.length===2 && classes.toggle('hidden')===false && !classes.contains('hidden');",
        "classes.toggle('hidden',true);",
        "p.className=hidden&&same?'hidden':'old';",
        "p.setAttribute('data-test',classes.value);",
        "</script>"
    ),800,600);
    assert!(!page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("MESSAGE-422")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m422_element_remove_and_reparent_preserve_node_refs() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<main id='host'><p id='old'>DELETE-422</p></main><script>",
            "var p=document.getElementById('old');",
            "var host=document.getElementById('host');",
            "p.remove();",
            "var detached=document.getElementById('old')===null && p.parentNode===null;",
            "host.appendChild(p);",
            "if(detached && p.parentNode===host && document.getElementById('old')===p)",
            " p.textContent='REMOVED-AND-RETURNED-422';",
            "</script>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("REMOVED-AND-RETURNED-422")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m421_detach_reattach_subtree_restores_nested_id_same_script() {
    let mut engine = Engine::new();
    let page=engine.set_html_page(concat!(
        "<main id='host'><section id='parent'><p id='grandchild'>BEFORE-REATTACH-421</p></section></main>",
        "<script>",
        "var host=document.getElementById('host');",
        "var parent=document.getElementById('parent');",
        "var grandchild=document.getElementById('grandchild');",
        "host.removeChild(parent);",
        "var hidden=document.getElementById('grandchild')===null;",
        "host.appendChild(parent);",
        "var restored=document.getElementById('grandchild')===grandchild;",
        "grandchild.textContent=hidden && restored?'RESTORED-NESTED-421':'STILL-MISSING';",
        "</script>"
    ),800,600);
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("RESTORED-NESTED-421")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m421_subtree_removal_hides_descendant_ids_in_same_script() {
    let mut engine = Engine::new();
    let page=engine.set_html_page(concat!(
        "<main id='root'><section id='middle'><p id='descendant'>CHILD-BEFORE-421</p></section></main>",
        "<script>",
        "var root=document.getElementById('root');",
        "var middle=document.getElementById('middle');",
        "var child=document.getElementById('descendant');",
        "root.removeChild(middle);",
        "if(document.getElementById('descendant')!==null) root.textContent='BAD-STILL-FOUND';",
        "else root.textContent='GONE-SUBTREE-421';",
        "</script>"
    ),800,600);
    assert!(!page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("BAD-STILL-FOUND")
    )));
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("GONE-SUBTREE-421")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m421_timer_style_and_text_insert_reflows_authoritative_dom() {
    let mut engine = Engine::new();
    let initial = engine.set_html_page(
        concat!(
            "<main id='host'><p id='message'>BEFORE-TIMER-421</p></main>",
            "<script>setTimeout(function(){",
            "var message=document.getElementById('message');",
            "message.setAttribute('style','display:none');",
            "var text=document.createTextNode('TIMER-TEXT-421');",
            "document.getElementById('host').appendChild(text);",
            "},0);</script>"
        ),
        800,
        600,
    );
    assert!(initial.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("BEFORE-TIMER-421")
    )));
    let updated = engine
        .tick_timers(800, 600)
        .expect("timer should mutate tree and style");
    assert!(!updated.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("BEFORE-TIMER-421")
    )));
    assert!(updated.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("TIMER-TEXT-421")
    )));
}

#[test]
fn m421_create_text_node_append_and_edit_repaints() {
    let mut engine = Engine::new();
    let page=engine.set_html_page(concat!(
        "<main id='host'></main><script>",
        "var text=document.createTextNode('REAL-TEXT-421');",
        "var same=text.nodeType===3 && text.data==='REAL-TEXT-421' && text.nodeValue==='REAL-TEXT-421';",
        "var parent=document.getElementById('host');",
        "var returned=parent.appendChild(text);",
        "text.data=same && returned===text ? 'EDITED-TEXT-421' : 'BROKEN-TEXT';",
        "</script>"
    ),800,600);
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("EDITED-TEXT-421")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m421_text_node_can_be_removed_and_reinserted() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<main id='host'></main><script>",
            "var p=document.getElementById('host');",
            "var t=document.createTextNode('TEXT-MOVED-421');",
            "p.appendChild(t);",
            "var x=p.removeChild(t);",
            "p.insertBefore(x,null);",
            "</script>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("TEXT-MOVED-421")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m421_remove_and_reinsert_existing_node_preserves_identity_and_paint() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<main id='host'><p id='first'>FIRST-421</p><p id='second'>SECOND-421</p></main>",
            "<script>",
            "var host=document.getElementById('host');",
            "var first=document.getElementById('first');",
            "var second=document.getElementById('second');",
            "var returned=host.removeChild(first);",
            "var detached=(returned===first && document.getElementById('first')===null);",
            "var again=host.insertBefore(first, second);",
            "if(detached && again===first && document.getElementById('first')===first) ",
            " first.textContent='M421-REINSERTED';",
            "</script>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M421-REINSERTED")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m421_remove_node_really_removes_visible_pixels() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<main id='host'><strong id='to-remove'>DISAPPEAR-421</strong><p>STAYS-421</p></main>",
            "<script>",
            "var parent=document.getElementById('host');",
            "var remove=document.getElementById('to-remove');",
            "parent.removeChild(remove);",
            "</script>"
        ),
        800,
        600,
    );
    assert!(!page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("DISAPPEAR-421")
    )));
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("STAYS-421")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m421_attributes_update_real_styles_and_lookup() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<main id='host'><p id='message' class='old'>HIDDEN-421</p><p>VISIBLE-421</p></main>",
            "<script>",
            "var node=document.getElementById('message');",
            "var old=node.getAttribute('class');",
            "node.setAttribute('class','new');",
            "node.setAttribute('data-test','OK');",
            "node.setAttribute('style','display:none');",
            "var correct=old==='old' && node.getAttribute('class')==='new' ",
            " && node.getAttribute('data-test')==='OK' && node.getAttribute('missing')===null;",
            "node.removeAttribute('data-test');",
            "var gone=node.getAttribute('data-test')===null;",
            "if(!correct || !gone) node.removeAttribute('style');",
            "</script>"
        ),
        800,
        600,
    );
    assert!(!page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("HIDDEN-421")
    )));
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("VISIBLE-421")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m421_invalid_reference_does_not_move_existing_child() {
    let mut engine = Engine::new();
    let page=engine.set_html_page(concat!(
        "<div id='host'><p id='one'>ONE-421</p></div><div id='other'><p id='two'>TWO-421</p></div>",
        "<script>",
        "var a=document.getElementById('host');",
        "var one=document.getElementById('one');",
        "var two=document.getElementById('two');",
        "var caught=false;",
        "try{a.insertBefore(one,two)}catch(e){caught=true}",
        "if(caught) one.textContent='INVALID-REF-SAFE-421';",
        "</script>"
    ),800,600);
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("INVALID-REF-SAFE-421")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m420_create_append_and_late_lookup_really_repaint_dom() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<div id='host'>START</div>",
            "<script>",
            "var parent=document.getElementById('host');",
            "var child=document.createElement('p');",
            "child.id='inserted';",
            "child.textContent='DYNAMIC-VISIBLE';",
            "var absent=document.getElementById('inserted')===null;",
            "var added=parent.appendChild(child);",
            "var same=added===child && document.getElementById('inserted')===child;",
            "var stable=parent===document.getElementById('host');",
            "</script>",
            "<script>",
            "if (absent && same && stable && document.getElementById('inserted')===child) ",
            "  child.textContent='M420-RENDERED';",
            "</script>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M420-RENDERED")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
    assert!(engine.active_script_report().unwrap().mutations >= 3);
    let page = engine.reflow(360, 500).unwrap();
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M420-RENDERED")
    )));
}

#[test]
fn m420_head_script_timer_gets_body_after_tree_builder_finishes() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<script>",
            "var initiallyAbsent=document.body===null;",
            "setTimeout(function(){",
            " var node=document.createElement('p');",
            " node.textContent='HEAD-BODY-M420';",
            " if(initiallyAbsent) document.body.appendChild(node);",
            "},0);",
            "</script><body><p>BASE</p>"
        ),
        800,
        600,
    );
    assert!(!page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("HEAD-BODY-M420")
    )));
    let updated = engine
        .tick_timers(800, 600)
        .expect("head timer must append after body exists");
    assert!(updated.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("HEAD-BODY-M420")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m420_dynamic_dom_timer_mutations_are_persistent_and_visible() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<section id='host'></section><script>",
            "var dynamic=document.createElement('b');",
            "dynamic.id='late';",
            "document.getElementById('host').appendChild(dynamic);",
            "setTimeout(function(){ dynamic.textContent='M420-TIMER'; },0);",
            "</script>"
        ),
        800,
        600,
    );
    assert!(!page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M420-TIMER")
    )));
    let updated = engine
        .tick_timers(800, 600)
        .expect("timer mutates real DOM");
    assert!(updated.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("M420-TIMER")
    )));
}

#[test]
fn m420_detached_nodes_can_be_nested_then_appended() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<main id='target'></main><script>",
            "var parent=document.createElement('section');",
            "var child=document.createElement('em');",
            "child.id='nested-child';",
            "child.textContent='NESTED-420';",
            "parent.appendChild(child);",
            "var wasDetached=document.getElementById('nested-child')===null;",
            "document.getElementById('target').appendChild(parent);",
            "var isAttached=document.getElementById('nested-child')===child;",
            "if(!wasDetached || !isAttached) child.textContent='BROKEN-DOM';",
            "</script>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("NESTED-420")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn m419_string_and_array_methods_reach_native_repaint() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<p id='result'>Loading</p><script>",
            "var letters=['A','B'];",
            "letters.push('C');",
            "var suffix=letters.pop();",
            "var label='ok'.charAt(0)+suffix;",
            "var caught=false;",
            "try{JSON.parse('{bad')}catch(e){caught=e instanceof SyntaxError;}",
            "if(caught && letters.length===2)",
            " document.getElementById('result').textContent='M419-'+label;",
            "</script>",
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text{text,..} if text.contains("M419-oC")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn typeof_conditional_and_std_static_functions_repaint_native_pixels() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<p id='result'>Loading</p><script>",
            "var samples=[1,2,3];",
            "var ok=typeof unknownGlobal==='undefined' && ",
            "typeof samples==='object' && typeof JSON.parse==='function' && ",
            "Array.isArray(samples) && !Array.isArray({}) && ",
            "Number.isFinite(2) && !Number.isFinite('2') && ",
            "Object.is(NaN,NaN) && !Object.is(0,-0);",
            "var text=ok ? 'TYPED-OK' : 'FAIL';",
            "document.getElementById('result').textContent=text;",
            "</script>"
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("TYPED-OK")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn primitive_coercion_updates_native_pixels() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<p id='out'>Not ready</p><script>",
            "var obj={valueOf:function(){return 37;}};",
            "var boxed=new Number(5);",
            "var ok=boxed instanceof Number && boxed !== 5 && boxed==5;",
            "if(ok) document.getElementById('out').textContent='OP-'+(obj+boxed);",
            "</script>",
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("OP-42")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn parser_blocking_inline_script_cannot_observe_future_dom_nodes() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        "<p id='earlier'>Earlier</p>         <script>var visible='WRONG';         if (document.getElementById('later') == null) { visible='NOT-YET'; }         document.getElementById('earlier').textContent='FIRST-UPDATED';</script>         <p id='later'>Later</p>         <script>document.getElementById('later').textContent=visible;</script>",
        800,
        600,
    );
    let visible = |text: &str| {
        page.commands.iter().any(|cmd| {
            matches!(
                cmd, op_paint::PaintCommand::Text {text: value,..} if value.contains(text)
            )
        })
    };
    assert!(visible("FIRST-UPDATED"));
    assert!(
        visible("NOT-YET"),
        "a parser-blocking script must not see future nodes"
    );
    assert!(!visible("WRONG"));
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (report.executed, report.failed, report.mutations),
        (2, 0, 2)
    );
}

#[test]
fn parser_snapshot_refresh_sees_later_elements_and_retains_globals() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        "<div id='box'><script>var partial=document.getElementById('box').textContent;         var suffix='READY';</script>Tail</div>         <p id='later'>Before</p>         <script>document.getElementById('later').textContent=partial+suffix;</script>",
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("READY")
    )));
    assert!(!page.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("TailREADY")
    )));
    assert_eq!(engine.active_script_report().unwrap().executed, 2);
}
#[test]
fn local_external_script_preserves_mixed_document_order_and_reflow() {
    let root = std::env::temp_dir().join(format!("opbrowser-js-m42-{}-v1", std::process::id(),));
    std::fs::create_dir_all(&root).unwrap();
    let page_path = root.join("index.html");
    let script_path = root.join("stage.js");
    std::fs::write(
        &script_path,
        "var state = state + 'B'; document.getElementById('result').textContent = state;",
    )
    .unwrap();
    std::fs::write(
        &page_path,
        concat!(
            "<!doctype html><p id='result'>Before</p>",
            "<script>var state='A';</script>",
            "<script src='stage.js'></script>",
            "<script>document.getElementById('result').textContent = state+'C';</script>",
        ),
    )
    .unwrap();

    let mut engine = Engine::new();
    let page = engine
        .navigate(&page_path.display().to_string(), 800, 600)
        .unwrap();
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (
            report.executed,
            report.failed,
            report.skipped,
            report.mutations
        ),
        (3, 0, 0, 2),
        "{report:?}"
    );
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("ABC")
    )));
    assert!(
        engine
            .reflow(300, 400)
            .unwrap()
            .display_list
            .commands
            .iter()
            .any(|cmd| matches!(
                cmd, op_paint::PaintCommand::Text {text,..} if text.contains("ABC")
            ))
    );
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn missing_external_script_does_not_abort_following_inline_script() {
    let root = std::env::temp_dir().join(format!(
        "opbrowser-js-m42-missing-{}-v1",
        std::process::id()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let path = root.join("index.html");
    std::fs::write(
        &path,
        concat!(
            "<p id='result'>Before</p>",
            "<script src='missing.js'></script>",
            "<script>document.getElementById('result').textContent='Recovered';</script>",
        ),
    )
    .unwrap();
    let mut engine = Engine::new();
    let page = engine
        .navigate(&path.display().to_string(), 800, 600)
        .unwrap();
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (report.executed, report.failed, report.mutations),
        (1, 1, 1)
    );
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("Recovered")
    )));
    std::fs::remove_dir_all(&root).unwrap();
}

#[test]
fn external_script_resource_type_uses_request_filter_before_loading() {
    let mut network = NetworkContext::default();
    let loaded = network
        .request_filter_mut()
        .import_adblock_rules("||blocked.example^$script");
    assert_eq!(loaded.accepted, 1);
    let error = network
        .load_script_for_page(
            "https://blocked.example/bundle.js",
            "https://blocked.example/page.html",
            1024,
        )
        .unwrap_err();
    assert!(
        matches!(error, LoadError::BlockedRequest { .. }),
        "{error:?}"
    );
}

#[test]
fn cross_origin_script_is_never_fetched() {
    let network = NetworkContext::default();
    let error = network
        .load_script_for_page(
            "https://unrelated.example/evil.js",
            "https://my-site.example/index.html",
            1024,
        )
        .unwrap_err();
    assert!(matches!(error, LoadError::InvalidLink(_)), "{error:?}");
}

#[test]
#[cfg(windows)]
fn winhttp_external_javascript_roundtrip_executes_to_dom() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        for expected in ["/index.html", "/src/app.js"] {
            let began = Instant::now();
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(began.elapsed() < Duration::from_secs(8));
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(3)))
                .unwrap();
            let mut buf = [0u8; 8192];
            let bytes = stream.read(&mut buf).unwrap();
            let request = String::from_utf8_lossy(&buf[..bytes]);
            assert!(
                request.starts_with(&format!("GET {expected} ")),
                "{request:?}"
            );
            let (mime, body) = if expected == "/index.html" {
                (
                    "text/html",
                    concat!(
                        "<p id='target'>Before</p>",
                        "<script>var text='net-';</script>",
                        "<script src='src/app.js'></script>",
                        "<script>document.getElementById('target').textContent=text+'ok';</script>"
                    ),
                )
            } else {
                ("text/javascript", "text=text+'loaded-';")
            };
            write!(stream,
                "HTTP/1.1 200 OK\r\nContent-Type: {mime};charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()).unwrap();
            stream.flush().unwrap();
        }
    });
    let mut engine = Engine::new();
    let page = engine
        .navigate(&format!("{origin}/index.html"), 800, 600)
        .unwrap();
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("net-loaded-ok")
    )));
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (report.executed, report.failed, report.skipped),
        (3, 0, 0),
        "{report:?}"
    );
    server.join().unwrap();
}

#[test]
fn defer_scripts_wait_for_complete_dom_and_execute_in_source_order() {
    let root = std::env::temp_dir().join(format!("opbrowser-js-m47-defer-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let page = root.join("index.html");
    std::fs::write(
        root.join("first.js"),
        "if (document.getElementById('later') == null) { throw 'no later DOM'; } trace=trace+'D';",
    )
    .unwrap();
    std::fs::write(
        root.join("second.js"),
        "trace=trace+'E'; document.getElementById('out').textContent=trace;",
    )
    .unwrap();
    std::fs::write(
        &page,
        concat!(
            "<p id='out'>Before</p>",
            "<script>var trace='A';</script>",
            "<script defer src='first.js'></script>",
            "<script>trace=trace+'B';</script>",
            "<script defer src='second.js'></script>",
            "<p id='later'>Present after parse</p>",
            "<script>trace=trace+'C';</script>",
        ),
    )
    .unwrap();
    let mut engine = Engine::new();
    let rendered = engine
        .navigate(&page.display().to_string(), 800, 600)
        .unwrap();
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (report.executed, report.failed, report.skipped),
        (5, 0, 0),
        "{report:?}"
    );
    assert!(rendered.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("ABCDE")
    )));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn async_external_script_executes_in_retained_vm() {
    let root = std::env::temp_dir().join(format!("opbrowser-js-m47-async-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let page = root.join("index.html");
    std::fs::write(
        root.join("async.js"),
        "document.getElementById('out').textContent='ASYNC-READY';",
    )
    .unwrap();
    std::fs::write(
        &page,
        concat!(
            "<p id='out'>Before</p>",
            "<script async src='async.js'></script>",
            "<p id='later'>Later</p>",
        ),
    )
    .unwrap();
    let mut engine = Engine::new();
    let rendered = engine
        .navigate(&page.display().to_string(), 800, 600)
        .unwrap();
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (report.executed, report.failed, report.skipped),
        (1, 0, 0),
        "{report:?}"
    );
    assert!(rendered.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("ASYNC-READY")
    )));
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn inline_async_and_defer_flags_do_not_defer_classic_inline_script() {
    let mut engine = Engine::new();
    let rendered = engine.set_html_page(
        "<p id='out'>Before</p><script async defer>var value='SYNC'; \
         if (document.getElementById('later') != null) { value='WRONG'; } \
         document.getElementById('out').textContent=value;</script><p id='later'>After</p>",
        800,
        600,
    );
    assert!(rendered.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("SYNC")
    )));
    assert!(!rendered.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("WRONG")
    )));
    assert_eq!(engine.active_script_report().unwrap().executed, 1);
}

#[test]
#[cfg(windows)]
fn async_scripts_execute_in_download_completion_order_not_tag_order() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    use std::time::{Duration, Instant};

    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let mut workers = Vec::new();
        for _ in 0..3 {
            let began = Instant::now();
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(began.elapsed() < Duration::from_secs(8));
                        std::thread::sleep(Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            workers.push(std::thread::spawn(move || {
                stream.set_read_timeout(Some(Duration::from_secs(3))).unwrap();
                let mut buffer = [0_u8; 8192];
                let count = stream.read(&mut buffer).unwrap();
                let request = String::from_utf8_lossy(&buffer[..count]);
                let (mime, body) = if request.starts_with("GET /index.html ") {
                    ("text/html", concat!(
                        "<p id='out'>Before</p>",
                        "<script>var finished='';</script>",
                        "<script async src='slow.js'></script>",
                        "<script async src='fast.js'></script>",
                    ))
                } else if request.starts_with("GET /slow.js ") {
                    std::thread::sleep(Duration::from_millis(500));
                    ("text/javascript", "finished=finished+'S';document.getElementById('out').textContent=finished;")
                } else {
                    assert!(request.starts_with("GET /fast.js "), "{request:?}");
                    ("text/javascript", "finished=finished+'F';document.getElementById('out').textContent=finished;")
                };
                write!(stream,
                    "HTTP/1.1 200 OK\r\nContent-Type: {mime};charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()).unwrap();
                stream.flush().unwrap();
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
    });
    let mut engine = Engine::new();
    let rendered = engine
        .navigate(&format!("{origin}/index.html"), 800, 600)
        .unwrap();
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (report.executed, report.failed, report.skipped),
        (3, 0, 0),
        "{report:?}"
    );
    assert!(rendered.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("FS")
    )));
    server.join().unwrap();
}

#[test]
fn lifecycle_events_follow_parser_completion_and_change_ready_state() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<p id='output'>Before</p>",
            "<script>",
            "var history='';",
            "document.addEventListener('readystatechange',function(e){",
            "history=history+'R'+document.readyState+'-'+e.eventPhase+';';",
            "});",
            "document.addEventListener('DOMContentLoaded',function(e){",
            "if (e.target == document && e.currentTarget == document && this == document) {",
            "history=history+'D'+document.readyState+';';",
            "}",
            "});",
            "window.addEventListener('load',function(e){",
            "if (this == window && e.currentTarget == window && e.target == document) {",
            "history=history+'L'+document.readyState+';';",
            "document.getElementById('output').textContent=history;",
            "}",
            "});",
            "</script>",
            "<p id='later'>Parsed after listener installation</p>",
        ),
        800,
        600,
    );
    let report = engine.active_script_report().unwrap();
    assert_eq!(
        (report.executed, report.failed, report.mutations),
        (1, 0, 1),
        "{report:?}"
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..}
            if text.contains("Rinteractive-2;Dinteractive;Rcomplete-2;Lcomplete;")
    )));
}

#[test]
fn lifecycle_property_handlers_and_listener_removal_work() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        concat!(
            "<p id='output'>Before</p>",
            "<script>",
            "var history='';",
            "var gone=function(){history=history+'BAD';};",
            "document.addEventListener('DOMContentLoaded',gone);",
            "document.removeEventListener('DOMContentLoaded',gone);",
            "document.onreadystatechange=function(){history=history+'R'+document.readyState+';';};",
            "window.onload=function(){",
            "document.getElementById('output').textContent=history+'ONLOAD';",
            "};",
            "document.readyState='untrusted';",
            "if (document.readyState != 'loading') { throw 'readyState must be read-only'; }",
            "</script>",
        ),
        800,
        600,
    );
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..}
            if text.contains("Rinteractive;Rcomplete;ONLOAD")
    )));
    assert!(!page.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("BAD")
    )));
    let report = engine.active_script_report().unwrap();
    assert_eq!((report.executed, report.failed), (1, 0), "{report:?}");
}

#[test]
fn deferred_script_registers_dom_content_loaded_before_dispatch() {
    let root =
        std::env::temp_dir().join(format!("opbrowser-js-m48-lifecycle-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let page = root.join("index.html");
    std::fs::write(
        root.join("deferred.js"),
        concat!(
            "if (document.readyState != 'interactive') { throw 'not interactive'; }",
            "document.addEventListener('DOMContentLoaded',function(){",
            "document.getElementById('out').textContent='DEFER-LOADED-'+document.readyState;",
            "});",
        ),
    )
    .unwrap();
    std::fs::write(
        &page,
        concat!(
            "<p id='out'>Before</p>",
            "<script defer src='deferred.js'></script>",
            "<p id='later'>After parser</p>",
        ),
    )
    .unwrap();
    let mut engine = Engine::new();
    let rendered = engine
        .navigate(&page.display().to_string(), 800, 600)
        .unwrap();
    assert!(rendered.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..}
            if text.contains("DEFER-LOADED-interactive")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn timers_fire_only_after_initial_page_load_when_worker_ticks() {
    let mut engine = Engine::new();
    let initial = engine.set_html_page(
        "<p id='output'>Before</p><script>\
         var history='script-';\
         window.addEventListener('load',function(){history=history+'load-';});\
         setTimeout(function(){\
         document.getElementById('output').textContent=history+document.readyState;\
         },0);</script>",
        800,
        600,
    );
    assert!(initial.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("Before")
    )));
    assert!(engine.next_timer_wait().is_some());
    let painted = engine.tick_timers(800, 600).expect("timer updated DOM");
    assert!(painted.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..}
            if text.contains("script-load-complete")
    )));
    assert!(engine.next_timer_wait().is_none());
    assert_eq!(engine.active_script_report().unwrap().mutations, 1);
}

#[test]
fn timer_cancellation_and_callback_arguments_preserve_single_vm() {
    let mut engine = Engine::new();
    engine.set_html_page(
        "<p id='output'>Before</p><script>\
         var first=setTimeout(function(a,b){\
         clearTimeout(second);\
         document.getElementById('output').textContent=a+b;\
         },0,'YES','-OK');\
         var second=setTimeout(function(){\
         document.getElementById('output').textContent='SHOULD-NOT-RUN';\
         },0);</script>",
        800,
        600,
    );
    let page = engine
        .tick_timers(800, 600)
        .expect("first timer changes text");
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("YES-OK")
    )));
    assert!(engine.next_timer_wait().is_none());
    assert!(engine.tick_timers(800, 600).is_none());
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
}

#[test]
fn delayed_timers_and_failed_callbacks_do_not_block_next_task() {
    let mut engine = Engine::new();
    engine.set_html_page(
        "<p id='output'>Before</p><script>\
         setTimeout(function(){throw 'error';},0);\
         setTimeout(function(){document.getElementById('output').textContent='RECOVER';},25);\
         </script>",
        800,
        600,
    );
    assert!(engine.tick_timers(800, 600).is_none());
    assert_eq!(engine.active_script_report().unwrap().failed, 1);
    std::thread::sleep(std::time::Duration::from_millis(40));
    let page = engine.tick_timers(800, 600).expect("later timer fired");
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("RECOVER")
    )));
    assert_eq!(engine.active_script_report().unwrap().failed, 1);
}

#[test]
fn replacing_document_discards_timer_callbacks_from_old_page() {
    let mut engine = Engine::new();
    engine.set_html_page(
        "<p id='output'>Old</p>\
         <script>setTimeout(function(){\
         document.getElementById('output').textContent='WRONG';\
         },0);</script>",
        800,
        600,
    );
    engine.set_html_page("<p id='output'>New</p>", 800, 600);
    assert!(engine.next_timer_wait().is_none());
    assert!(engine.tick_timers(800, 600).is_none());
    assert!(
        engine
            .reflow(800, 600)
            .unwrap()
            .display_list
            .commands
            .iter()
            .any(|cmd| matches!(
                cmd, op_paint::PaintCommand::Text {text,..} if text.contains("New")
            ))
    );
}

#[test]
fn interval_self_cancel_runs_twice_without_orphaned_tasks() {
    let mut engine = Engine::new();
    engine.set_html_page(
        "<p id='out'>Waiting</p><script>\
         var ticks=0;var id=setInterval(function(){\
           ticks=ticks+1;\
           document.getElementById('out').textContent='Tick-'+ticks;\
           if(ticks==2){clearInterval(id);}\
         },4);</script>",
        800,
        600,
    );
    assert!(engine.tick_timers(800, 600).is_none());
    std::thread::sleep(std::time::Duration::from_millis(15));
    let page = engine.tick_timers(800, 600).expect("first interval");
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("Tick-1")
    )));
    std::thread::sleep(std::time::Duration::from_millis(15));
    let page = engine.tick_timers(800, 600).expect("second interval");
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("Tick-2")
    )));
    assert!(engine.next_timer_wait().is_none());
    assert!(engine.tick_timers(800, 600).is_none());
}

#[test]
fn clear_timeout_also_cancels_interval_and_clear_interval_cancels_timeout() {
    let mut engine = Engine::new();
    engine.set_html_page(
        "<p id='out'>Safe</p><script>\
         var id=setInterval(function(){document.getElementById('out').textContent='BAD';},4);\
         clearTimeout(id);\
         var once=setTimeout(function(){document.getElementById('out').textContent='BAD';},0);\
         clearInterval(once);</script>",
        800,
        600,
    );
    assert!(engine.next_timer_wait().is_none());
    std::thread::sleep(std::time::Duration::from_millis(8));
    assert!(engine.tick_timers(800, 600).is_none());
}

#[test]
fn microtask_checkpoint_is_fifo_and_runs_before_timer_macrotask() {
    let mut engine = Engine::new();
    let page = engine.set_html_page(
        "<p id='out'>Waiting</p><script>\
         var log='S';\
         setTimeout(function(){log=log+'T';document.getElementById('out').textContent=log;},0);\
         queueMicrotask(function(){log=log+'A';queueMicrotask(function(){\
            log=log+'B';document.getElementById('out').textContent=log;});});\
         queueMicrotask(function(){log=log+'C';});\
         </script>",
        800,
        600,
    );
    // FIFO microtasks: A, C, B (B was queued by A).
    assert!(page.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("SACB")
    )));
    let next = engine
        .tick_timers(800, 600)
        .expect("timer follows microtasks");
    assert!(next.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("SACBT")
    )));
}

#[test]
fn microtasks_from_timer_run_before_next_due_timer() {
    let mut engine = Engine::new();
    engine.set_html_page(
        "<p id='out'>Waiting</p><script>\
         var log='';\
         setTimeout(function(){log=log+'A';queueMicrotask(function(){log=log+'M';});},0);\
         setTimeout(function(){document.getElementById('out').textContent=log+'B';},0);\
         </script>",
        800,
        600,
    );
    let page = engine.tick_timers(800, 600).expect("two timers");
    assert!(page.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text {text,..} if text.contains("AMB")
    )));
}

#[test]
fn interval_does_not_survive_navigation() {
    let mut engine = Engine::new();
    engine.set_html_page("<script>setInterval(function(){},4);</script>", 800, 600);
    assert!(engine.next_timer_wait().is_some());
    engine.set_html_page("<p>New page</p>", 800, 600);
    assert!(engine.next_timer_wait().is_none());
}

#[test]
fn post_presentation_text_task_loads_local_resource_and_repaints() {
    let root = std::env::temp_dir().join(format!("opbrowser-m411-file-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let page = root.join("index.html");
    std::fs::write(root.join("hello.txt"), "LOCAL-RESOURCE").unwrap();
    std::fs::write(
        &page,
        concat!(
            "<p id='out'>Before</p>",
            "<script>opFetchText('hello.txt',function(body,error){",
            "if(error==null){document.getElementById('out').textContent=body;}else{document.getElementById('out').textContent=error;}",
            "});</script>"
        ),
    )
    .unwrap();
    let mut engine = Engine::new();
    let initial = engine
        .navigate(&page.display().to_string(), 800, 600)
        .unwrap();
    assert!(initial.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("Before")
    )));
    let mut done = false;
    for _ in 0..100 {
        if let Some(result) = engine.tick_timers(800, 600)
            && result.display_list.commands.iter().any(|cmd| {
                matches!(
                    cmd,op_paint::PaintCommand::Text{text,..} if text.contains("LOCAL-RESOURCE")
                )
            })
        {
            done = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(done, "network completion did not repaint page");
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn blocked_cross_origin_text_request_returns_error_without_network() {
    let root = std::env::temp_dir().join(format!("opbrowser-m411-cross-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let page = root.join("index.html");
    std::fs::write(
        &page,
        concat!(
            "<p id='out'>Before</p>",
            "<script>opFetchText('https://example.com/elsewhere.txt',function(body,error){",
            "if(body==null && error!=null){document.getElementById('out').textContent='BLOCKED';}else{document.getElementById('out').textContent='WRONG';}",
            "});</script>"
        ),
    )
    .unwrap();
    let mut engine = Engine::new();
    engine
        .navigate(&page.display().to_string(), 800, 600)
        .unwrap();
    let result = engine.tick_timers(800, 600).unwrap();
    assert!(result.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("BLOCKED")
    )));
    assert!(engine.next_timer_wait().is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[cfg(windows)]
fn winhttp_text_completion_runs_on_engine_after_first_render() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    listener.set_nonblocking(true).unwrap();
    let server = std::thread::spawn(move || {
        let mut workers = Vec::new();
        for _ in 0..2 {
            let start = std::time::Instant::now();
            let mut stream = loop {
                match listener.accept() {
                    Ok((stream, _)) => break stream,
                    Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                        assert!(start.elapsed().as_secs() < 8);
                        std::thread::sleep(std::time::Duration::from_millis(5));
                    }
                    Err(error) => panic!("{error}"),
                }
            };
            stream.set_nonblocking(false).unwrap();
            workers.push(std::thread::spawn(move||{
                stream.set_read_timeout(Some(std::time::Duration::from_secs(3))).unwrap();
                let mut buf=[0_u8;8192];
                let n=stream.read(&mut buf).unwrap();
                let req=String::from_utf8_lossy(&buf[..n]);
                let (mime,body)=if req.starts_with("GET /index.html ") {
                    ("text/html",concat!(
                        "<p id='out'>INITIAL</p>",
                        "<script>opFetchText('data.json',function(body,error){",
                        "if(error==null){document.getElementById('out').textContent=body;}else{document.getElementById('out').textContent=error;}",
                        "});</script>"
                    ))
                }else{
                    assert!(req.starts_with("GET /data.json "),"unexpected {req:?}");
                    std::thread::sleep(std::time::Duration::from_millis(180));
                    ("application/json","JSON-TEXT")
                };
                write!(stream,
                  "HTTP/1.1 200 OK\r\nContent-Type: {mime}; charset=UTF-8\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                  body.len()).unwrap();
                stream.flush().unwrap();
            }));
        }
        for worker in workers {
            worker.join().unwrap();
        }
    });
    let mut engine = Engine::new();
    let initial = engine
        .navigate(&format!("{origin}/index.html"), 800, 600)
        .unwrap();
    assert!(initial.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("INITIAL")
    )));
    assert!(engine.tick_timers(800, 600).is_none());
    let mut finished = false;
    for _ in 0..120 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        if let Some(page) = engine.tick_timers(800, 600)
            && page.display_list.commands.iter().any(|cmd| {
                matches!(
                    cmd,op_paint::PaintCommand::Text{text,..} if text.contains("JSON-TEXT")
                )
            })
        {
            finished = true;
            break;
        }
    }
    assert!(finished, "asynchronous network task did not repaint");
    server.join().unwrap();
}

#[test]
fn fetch_promise_resolves_a_local_response_and_repaints() {
    let root = std::env::temp_dir().join(format!("opbrowser-m412-fetch-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let page = root.join("index.html");
    std::fs::write(root.join("hello.txt"), "FETCH-RESOLVED").unwrap();
    std::fs::write(
        &page,
        concat!(
            "<p id='out'>Before fetch</p>",
            "<script>fetch('hello.txt').then(function(response){",
            "if(!response.ok || response.status!==200)throw 'bad response';",
            "return response.text();",
            "}).then(function(body){",
            "document.getElementById('out').textContent=body;",
            "}).catch(function(error){document.getElementById('out').textContent='ERROR';});",
            "</script>"
        ),
    )
    .unwrap();
    let mut engine = Engine::new();
    let initial = engine
        .navigate(&page.display().to_string(), 800, 600)
        .unwrap();
    assert!(initial.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text{text,..} if text.contains("Before fetch")
    )));
    let mut resolved = false;
    for _ in 0..100 {
        if let Some(page) = engine.tick_timers(800, 600)
            && page.display_list.commands.iter().any(|cmd| {
                matches!(
                    cmd, op_paint::PaintCommand::Text{text,..} if text.contains("FETCH-RESOLVED")
                )
            })
        {
            resolved = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(resolved, "fetch promise did not repaint the retained page");
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn json_response_renders_object_fields_after_async_get() {
    let root = std::env::temp_dir().join(format!("opbrowser-m415-json-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let page = root.join("index.html");
    std::fs::write(
        root.join("data.json"),
        r#"{"message":"JSON-REPAINT","count":42}"#,
    )
    .unwrap();
    std::fs::write(
        &page,
        concat!(
            "<p id='out'>Loading JSON</p><script>",
            "fetch('data.json').then(function(response){return response.json();})",
            ".then(function(data){document.getElementById('out').textContent=",
            "data.message+'-'+data.count;})",
            ".catch(function(e){document.getElementById('out').textContent='ERROR:'+e.message;});",
            "</script>"
        ),
    )
    .unwrap();
    let mut engine = Engine::new();
    engine
        .navigate(&page.display().to_string(), 800, 600)
        .unwrap();
    let mut rendered = false;
    for _ in 0..100 {
        if let Some(display) = engine.tick_timers(800, 600)
            && display.display_list.commands.iter().any(|cmd| {
                matches!(
                    cmd,op_paint::PaintCommand::Text{text,..} if text.contains("JSON-REPAINT-42")
                )
            })
        {
            rendered = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        rendered,
        "JSON response did not repaint the retained native page"
    );
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn fetch_promise_cross_origin_is_rejected_by_request_policy() {
    let root = std::env::temp_dir().join(format!("opbrowser-m412-cross-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let page = root.join("index.html");
    std::fs::write(
        &page,
        concat!(
            "<p id='out'>Waiting</p>",
            "<script>fetch('https://example.com/outside.txt')",
            ".then(function(){document.getElementById('out').textContent='UNSAFE';})",
            ".catch(function(){document.getElementById('out').textContent='REJECTED';});</script>"
        ),
    )
    .unwrap();
    let mut engine = Engine::new();
    engine
        .navigate(&page.display().to_string(), 800, 600)
        .unwrap();
    let result = engine
        .tick_timers(800, 600)
        .expect("rejected promise updates DOM");
    assert!(result.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text{text,..} if text.contains("REJECTED")
    )));
    assert!(engine.next_timer_wait().is_none());
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[cfg(windows)]
fn fetch_request_headers_are_sent_and_response_repaints_native_pixels() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for index in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .unwrap();
            let mut bytes = Vec::new();
            while !bytes.ends_with(b"\r\n\r\n") {
                let mut block = [0_u8; 2048];
                let n = stream.read(&mut block).unwrap();
                assert!(n > 0 && bytes.len() < 16 * 1024);
                bytes.extend_from_slice(&block[..n]);
            }
            let request = String::from_utf8_lossy(&bytes);
            let body = if index == 0 {
                assert!(request.starts_with("GET /index.html "));
                concat!(
                    "<p id='out'>WAITING</p><script>",
                    "var h=new Headers({'X-Client':'tested'});",
                    "fetch(new Request('data.txt',{headers:h,redirect:'error'}))",
                    ".then(function(r){return r.text();})",
                    ".then(function(t){document.getElementById('out').textContent=t;})",
                    ".catch(function(e){document.getElementById('out').textContent='FAIL:'+e.message;});",
                    "</script>"
                )
            } else {
                assert!(request.starts_with("GET /data.txt "));
                assert!(
                    request
                        .to_ascii_lowercase()
                        .contains("\r\nx-client: tested\r\n")
                );
                "HEADER-REACHED-SERVER"
            };
            let mime = if index == 0 {
                "text/html"
            } else {
                "text/plain"
            };
            write!(stream,
                "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            ).unwrap();
            stream.flush().unwrap();
        }
    });
    let mut engine = Engine::new();
    engine
        .navigate(&format!("{origin}/index.html"), 800, 600)
        .unwrap();
    let mut rendered = false;
    for _ in 0..150 {
        if let Some(page) = engine.tick_timers(800, 600)
            && page.display_list.commands.iter().any(|cmd| matches!(
                cmd,op_paint::PaintCommand::Text{text,..} if text.contains("HEADER-REACHED-SERVER")
            ))
        {
            rendered = true;
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        rendered,
        "fetch Request header/body not delivered to native text paint"
    );
    server.join().unwrap();
}

#[test]
#[cfg(windows)]
fn fetch_http_404_has_real_status_headers_and_body_after_first_paint() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let origin = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for index in 0..2 {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(3)))
                .unwrap();
            let mut bytes = [0_u8; 8192];
            let size = stream.read(&mut bytes).unwrap();
            let request = String::from_utf8_lossy(&bytes[..size]);
            let (status, headers, body) = if index == 0 {
                assert!(request.starts_with("GET /index.html "));
                (
                    "200 OK",
                    "Content-Type: text/html\r\n",
                    concat!(
                        "<p id='out'>Waiting</p><script>",
                        "fetch('missing.txt').then(function(r){",
                        "document.getElementById('out').textContent=",
                        "r.status+'|'+r.ok+'|'+r.statusText+'|'+",
                        "r.headers.get('X-MiXeD')+'|'+r.headers.has('set-cookie')+'|'+",
                        "r.url+'|'+r.redirected;",
                        "return r.text();",
                        "}).then(function(body){var el=document.getElementById('out');",
                        "el.textContent=el.textContent+'|'+body;",
                        "}).catch(function(e){document.getElementById('out').textContent='ERROR:'+e.message;});",
                        "</script>"
                    ),
                )
            } else {
                assert!(request.starts_with("GET /missing.txt "));
                (
                    "404 Not Found",
                    "Content-Type: text/plain\r\nX-Mixed: SomeValue\r\nSet-Cookie: session=secret\r\n",
                    "MISSING-BODY",
                )
            };
            write!(stream,
                "HTTP/1.1 {status}\r\n{headers}Content-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            ).unwrap();
            stream.flush().unwrap();
        }
    });
    let mut engine = Engine::new();
    let initial = engine
        .navigate(&format!("{origin}/index.html"), 800, 600)
        .unwrap();
    assert!(initial.display_list.commands.iter().any(|cmd| matches!(
        cmd, op_paint::PaintCommand::Text{text,..} if text.contains("Waiting")
    )));
    let mut rendered = false;
    for _ in 0..120 {
        if let Some(page) = engine.tick_timers(800, 600) {
            rendered = page.display_list.commands.iter().any(|cmd| matches!(
                cmd, op_paint::PaintCommand::Text{text,..} if
                text.contains(&format!("404|false|Not Found|SomeValue|false|{origin}/missing.txt|false|MISSING-BODY"))
            ));
            if rendered {
                break;
            }
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    assert!(
        rendered,
        "404 response metadata/body did not reach native pixels"
    );
    assert_eq!(engine.active_script_report().unwrap().failed, 0);
    server.join().unwrap();
}

#[test]
fn late_completion_from_previous_navigation_is_discarded() {
    let root = std::env::temp_dir().join(format!("opbrowser-m411-stale-{}", std::process::id()));
    std::fs::create_dir_all(&root).unwrap();
    let old = root.join("old.html");
    let new = root.join("new.html");
    std::fs::write(root.join("data.txt"), "STALE-BODY").unwrap();
    std::fs::write(
        &old,
        concat!(
            "<p id='out'>Old</p>",
            "<script>opFetchText('data.txt',function(body){",
            "document.getElementById('out').textContent=body;",
            "});</script>"
        ),
    )
    .unwrap();
    std::fs::write(&new, "<p id='out'>NEW-PAGE</p>").unwrap();
    let mut engine = Engine::new();
    engine
        .navigate(&old.display().to_string(), 800, 600)
        .unwrap();
    engine.tick_timers(800, 600);
    let initial = engine
        .navigate(&new.display().to_string(), 800, 600)
        .unwrap();
    assert!(initial.display_list.commands.iter().any(|cmd| matches!(
        cmd,op_paint::PaintCommand::Text{text,..} if text.contains("NEW-PAGE")
    )));
    std::thread::sleep(std::time::Duration::from_millis(50));
    assert!(engine.tick_timers(800, 600).is_none());
    assert!(
        engine
            .reflow(800, 600)
            .unwrap()
            .display_list
            .commands
            .iter()
            .any(|cmd| matches!(
                cmd,op_paint::PaintCommand::Text{text,..} if text.contains("NEW-PAGE")
            ))
    );
    assert!(engine.next_timer_wait().is_none());
    std::fs::remove_dir_all(root).unwrap();
}
