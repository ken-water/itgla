const {test,expect}=require('@playwright/test');

test('chart hover responds immediately across the plot, keyboard and touch work', async({page})=>{
 await page.goto('http://127.0.0.1:4174/admin');
 await page.getByLabel('Username').fill('admin');await page.getByLabel('Password').fill('test-password');await page.getByRole('button',{name:'Sign in',exact:true}).click();
 const svg=page.locator('#overview-trend > svg');await expect(svg).toBeVisible();
 const bounds=await svg.boundingBox();
 await page.mouse.move(bounds.x+bounds.width/2,bounds.y+bounds.height/2);
 const tip=svg.locator('.chart-tooltip');await expect(tip).toHaveAttribute('visibility','visible');
 await expect(tip).toContainText('2026-09-21');await expect(tip).toContainText('402');await expect(tip).toContainText('143');
 // Date selection depends on x, not on hitting a tiny dot.
 await page.mouse.move(bounds.x+bounds.width/2,bounds.y+bounds.height*.8);
 await expect(tip).toContainText('2026-09-21');
 await svg.focus();await page.keyboard.press('End');await expect(tip).toContainText('2026-09-22');
 await page.keyboard.press('ArrowLeft');await expect(tip).toContainText('2026-09-21');
 await page.keyboard.press('Escape');await expect(tip).toHaveAttribute('visibility','hidden');
 await svg.dispatchEvent('pointerdown',{clientX:bounds.x+bounds.width/2,clientY:bounds.y+bounds.height/2,pointerType:'touch'});
 await expect(tip).toHaveAttribute('visibility','visible');
 await page.mouse.move(bounds.x+bounds.width/2,bounds.y+bounds.height/2);
 await page.mouse.move(0,0);await expect(tip).toHaveAttribute('visibility','hidden');
});

test('all admin dates use full ISO date and 24 hour local time including midnight and feedback',async({page})=>{
 await page.goto('http://127.0.0.1:4174/admin');
 await page.getByLabel('Username').fill('admin');await page.getByLabel('Password').fill('test-password');await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await expect(page.locator('#freshness')).toHaveText(/^Latest data: \d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/);
 await expect(page.locator('#overview-events tbody tr').first().locator('td').first()).toHaveText(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/);
 const svg=page.locator('#overview-trend > svg');await expect(svg.locator('text').filter({hasText:/2026-09-20/})).toBeVisible();
 await page.locator('#overview-trend summary').click();await expect(page.locator('#overview-trend tbody tr').first().locator('td').first()).toHaveText('2026-09-20');
 expect(await page.evaluate(()=>window.formatAdminTime('2026-11-01T00:00:00'))).toBe('2026-11-01 00:00:00');
 await page.getByRole('tab',{name:'Feedback',exact:true}).click();await expect(page.locator('#feedback-list time').first()).toHaveText(/^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}$/);
});

test('stalled download details cannot hide funnel or charts; transient reads retry once',async({page})=>{
 await page.clock.install();let attempts=0;
 await page.route('**/admin/api/download-events-page?*',()=>{attempts++;});
 await page.goto('http://127.0.0.1:4174/admin');
 await page.getByLabel('Username').fill('admin');await page.getByLabel('Password').fill('test-password');await page.getByRole('button',{name:'Sign in',exact:true}).click();
 const pending=page.waitForRequest('**/admin/api/download-events-page?*');await page.getByRole('tab',{name:'Downloads',exact:true}).click();await pending;
 await expect(page.locator('#downloads-funnel')).toContainText('211');
 await expect(page.locator('#downloads-metrics .metric').nth(3)).toContainText('124');
 await expect(page.locator('#downloads-trend > svg')).toBeVisible();
 const retry=page.waitForRequest('**/admin/api/download-events-page?*');await page.clock.fastForward(15001);await retry;await page.clock.fastForward(15001);
 await expect(page.locator('#download-page-status')).toContainText('Retry this batch');
 await expect(page.locator('#dashboard-error')).toBeHidden();
 await expect(page.locator('#downloads-funnel')).toContainText('211');expect(attempts).toBe(2);
});

test('one transient report failure recovers automatically',async({page})=>{
 let attempts=0;
 await page.route('**/admin/api/download-summary?*',route=>++attempts===1?route.fulfill({status:503,contentType:'application/json',body:'{}'}):route.continue());
 await page.goto('http://127.0.0.1:4174/admin');
 await page.getByLabel('Username').fill('admin');await page.getByLabel('Password').fill('test-password');await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('tab',{name:'Downloads',exact:true}).click();await expect(page.locator('#downloads-funnel')).toContainText('211');expect(attempts).toBe(2);
 await expect(page.locator('#dashboard-error')).toBeHidden();
});

test('download pagination retains rows on failed batches and retries without duplicates',async({page})=>{
 let secondAttempts=0;
 const rows=(start)=>Array.from({length:25},(_,i)=>({id:String(start+i),occurred_at:'2026-09-25T14:25:00Z',event_name:'download_success',path:'/package-'+(start+i),status_code:200}));
 await page.route('**/admin/api/download-events-page?*',route=>{
  const cursor=new URL(route.request().url()).searchParams.get('cursor');
  if(!cursor)return route.fulfill({json:{events:rows(1),next_cursor:'25'}});
  if(++secondAttempts===1)return route.abort();
  return route.fulfill({json:{events:rows(26),next_cursor:null}});
 });
 await page.goto('http://127.0.0.1:4174/admin');
 await page.getByLabel('Username').fill('admin');await page.getByLabel('Password').fill('test-password');await page.getByRole('button',{name:'Sign in',exact:true}).click();
 await page.getByRole('tab',{name:'Downloads',exact:true}).click();await expect(page.locator('#downloads-events tbody tr')).toHaveCount(25);
 await page.getByRole('button',{name:'Load more',exact:true}).click();
 await expect(page.locator('#download-page-status')).toContainText('Existing requests are kept');
 await expect(page.locator('#downloads-events tbody tr')).toHaveCount(25);
 await page.getByRole('button',{name:'Retry this batch',exact:true}).click();
 await expect(page.locator('#downloads-events tbody tr')).toHaveCount(50);
 await expect(page.locator('#download-more')).toBeHidden();expect(secondAttempts).toBe(2);
 await page.getByRole('button',{name:'Refresh',exact:true}).click();
 await expect(page.locator('#downloads-events tbody tr')).toHaveCount(25);
});
